use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::{SCHED_EXT_STATE, SCXCTL_BIN};
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
        [Change::FileWritten { path, .. }, Change::SystemdUnit { unit, .. }] => {
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

#[test]
fn a_takeover_stops_the_other_scheduler_before_loading_lavd_and_keeps_it_as_undo() {
    // The user agreed to the takeover, so the loader clears the seat first and
    // records the previous scheduler whole: rollback switches back to cosmos
    // rather than stopping whatever is left.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .with_binary(SCXCTL_BIN)
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(crate::steps::constants::SCHED_EXT_OPS, "cosmos\n");
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    Loader::Scxctl.load(&mut apply).expect("loaded");

    assert_eq!(
        apply.recorded(),
        [Change::ScxScheduler {
            previous: Some("cosmos".to_owned())
        }]
    );
    let commands = runner.commands();
    let stop = commands
        .iter()
        .position(|command| command.contains("scxctl stop"))
        .expect("the owner is stopped first");
    let load = commands
        .iter()
        .position(|command| command.contains("scxctl start -s lavd"))
        .expect("lavd is loaded");
    assert!(stop < load, "{commands:?}");
}

#[test]
fn a_takeover_through_the_unit_stops_it_first_and_records_the_handover_before_the_dropin() {
    // The journal's newest-first undo removes the drop-in before it restarts
    // the unit, so the unit comes back running the scheduler its own config
    // names rather than ours.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .with_binary("systemctl")
        .with_file(SCX_UNIT_PATH, "[Unit]\n")
        .answering("systemctl is-enabled scx", "enabled")
        .answering("systemctl is-active scx", "active");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    Loader::Unit.load(&mut apply).expect("loaded");

    match apply.recorded() {
        [Change::SystemdUnit {
            unit,
            was_enabled: true,
            was_active: true,
        }, Change::FileWritten { path, .. }] => {
            assert_eq!(unit, SCX_SERVICE_NAME);
            assert_eq!(path, Path::new(SCX_UNIT_DROPIN));
        }
        other => panic!("expected a handover before the drop-in, got {other:?}"),
    }
    let commands = runner.commands();
    let stop = commands
        .iter()
        .position(|command| command.contains("systemctl stop scx"))
        .expect("the running unit is stopped first");
    let start = commands
        .iter()
        .position(|command| command.contains("systemctl enable --now scx"))
        .expect("the unit is re-enabled and started");
    assert!(stop < start, "{commands:?}");
}
