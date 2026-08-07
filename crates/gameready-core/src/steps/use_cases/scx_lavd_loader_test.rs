use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::SCXCTL_BIN;
use crate::steps::use_cases::scx_lavd::ScxLavd;

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open")
}

#[test]
fn scxctl_on_path_wins_whatever_the_family_packages() {
    let runner = MockRunner::new().with_binary(SCXCTL_BIN);
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    assert_eq!(Loader::detect(&cx), Loader::Scxctl);
}

#[test]
fn the_shipped_unit_is_used_when_there_is_no_scxctl() {
    // The Ubuntu PPA ships scx.service and no loader at all, so a step that
    // only knew scxctl would install 17 schedulers and then fail to run one.
    let runner = MockRunner::new().with_file(SCX_UNIT_PATH, "[Unit]\n");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    assert_eq!(Loader::detect(&cx), Loader::Unit);
}

#[test]
fn a_machine_with_neither_is_predicted_from_its_packaging() {
    // The plan screen has to name what will happen before anything is fetched,
    // and the two families install different mechanisms.
    let runner = MockRunner::new();

    let debian = SystemFacts::fixture(Family::Debian);
    assert_eq!(Loader::detect(&CoreCx::new(&debian, &runner)), Loader::Unit);

    let arch = SystemFacts::fixture(Family::Arch);
    assert_eq!(Loader::detect(&CoreCx::new(&arch, &runner)), Loader::Scxctl);
}

#[test]
fn only_the_unit_mechanism_survives_a_reboot() {
    // The one property that differs between them, and the one a user has to
    // read before agreeing.
    assert!(Loader::Unit.survives_reboot());
    assert!(!Loader::Scxctl.survives_reboot());
}

#[test]
fn loading_through_the_unit_writes_the_dropin_before_it_starts_anything() {
    // A run that dies between the two leaves a machine correct on the next
    // boot rather than correct now and wrong later.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_file(SCX_UNIT_PATH, "[Unit]\n");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    Loader::Unit.load(&mut apply).expect("loaded");

    match apply.recorded() {
        [
            Change::FileWritten { path, .. },
            Change::SystemdUnit { unit, .. },
        ] => {
            assert_eq!(path, Path::new(SCX_UNIT_DROPIN));
            assert_eq!(unit, SCX_SERVICE_NAME);
        }
        other => panic!("expected a drop-in then a unit start, got {other:?}"),
    }
}

#[test]
fn the_dropin_aims_the_shipped_unit_at_lavd_without_editing_the_package_file() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_file(SCX_UNIT_PATH, "[Unit]\n");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    Loader::Unit.load(&mut apply).expect("loaded");

    let written = runner.file(SCX_UNIT_DROPIN).expect("drop-in written");
    assert!(written.contains(SCX_SCHEDULER_OVERRIDE), "{written}");
    assert!(written.contains(SCX_LAVD_BIN), "{written}");
    // /etc/default/scx belongs to the package and is never touched.
    assert!(runner.file("/etc/default/scx").is_none());
}

#[test]
fn loading_through_scxctl_records_what_it_replaced_and_writes_no_file() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_binary(SCXCTL_BIN);
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    Loader::Scxctl.load(&mut apply).expect("loaded");

    assert_eq!(apply.recorded(), [Change::ScxScheduler { previous: None }]);
    assert!(runner.paths().is_empty(), "{:?}", runner.paths());
}
