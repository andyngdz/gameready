use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::{Apt, Pacman};
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::SCX_PPA_ORIGIN;

/// An Ubuntu box with no scx anywhere, which is every Ubuntu box by default.
fn ubuntu_without_scx() -> MockRunner {
    MockRunner::new()
        .failing("dpkg-query --showformat=${Version} --show scx")
        .failing("apt-cache show scx")
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open")
}

#[test]
fn a_system_that_is_not_apt_never_needs_this() {
    let runner = MockRunner::new();
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Pacman);

    match ScxPpa.probe(&cx).expect("probed") {
        Probe::NotApplicable { reason } => assert!(reason.contains("extra"), "{reason}"),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn an_ubuntu_box_without_scx_needs_the_repository() {
    let runner = ubuntu_without_scx();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert!(matches!(
        ScxPpa.probe(&cx).expect("probed"),
        Probe::Applicable
    ));
}

#[test]
fn a_user_who_added_the_ppa_by_hand_is_left_alone() {
    // The goal is that scx resolves, not that gameready's own file exists.
    let runner = MockRunner::new()
        .failing("dpkg-query --showformat=${Version} --show scx")
        .answering("apt-cache show scx", "Package: scx\n");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert!(matches!(
        ScxPpa.probe(&cx).expect("probed"),
        Probe::AlreadyApplied { .. }
    ));
}

#[test]
fn the_plan_shows_the_pin_that_holds_the_repository_back() {
    let runner = ubuntu_without_scx();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let plan = ScxPpa.plan(&cx).expect("planned");
    match &plan.actions[..] {
        [
            PlannedAction::CreateFile { path, contents },
            PlannedAction::RunCommand { display },
        ] => {
            assert_eq!(path, SCX_PPA_PIN);
            assert!(contents.contains("Pin-Priority: 1"), "{contents}");
            assert!(display.contains(SCX_PPA), "{display}");
        }
        other => panic!("expected a pin then a repository add, got {other:?}"),
    }
}

#[test]
fn the_pin_lets_the_ppa_supply_scx_and_refuses_it_everything_else() {
    // Without this a third-party repository can replace any package on the
    // system, which is the risk the step exists to bound.
    let pin = scx_ppa_pin::body("scx");

    assert!(pin.contains("Package: *"), "{pin}");
    assert!(pin.contains("Pin-Priority: 1"), "{pin}");
    assert!(pin.contains("Package: scx"), "{pin}");
    assert!(pin.contains("Pin-Priority: 500"), "{pin}");
    assert!(pin.contains(SCX_PPA_ORIGIN), "{pin}");
}

#[test]
fn apply_pins_before_it_adds_so_there_is_never_an_unpinned_window() {
    // A run interrupted between the two would otherwise leave the PPA
    // configured at full priority for good.
    let dir = TempDir::new().expect("temp dir");
    let runner = ubuntu_without_scx();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxPpa::id_const(), &runner, &mut journal);

    ScxPpa.apply(&mut apply).expect("applied");

    match apply.recorded() {
        [
            Change::FileWritten { path, .. },
            Change::AptRepository { spec },
        ] => {
            assert_eq!(path, Path::new(SCX_PPA_PIN));
            assert_eq!(spec, SCX_PPA);
        }
        other => panic!("expected a pin then a repository, got {other:?}"),
    }
}

#[test]
fn the_written_pin_carries_the_marker_doctor_finds_it_by() {
    let dir = TempDir::new().expect("temp dir");
    let runner = ubuntu_without_scx();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxPpa::id_const(), &runner, &mut journal);

    ScxPpa.apply(&mut apply).expect("applied");

    let written = runner.file(SCX_PPA_PIN).expect("pin written");
    assert!(
        written.starts_with(crate::steps::MANAGED_HEADER),
        "{written}"
    );
}

#[test]
fn rollback_takes_the_repository_off_before_it_drops_the_pin() {
    let dir = TempDir::new().expect("temp dir");
    let runner = ubuntu_without_scx();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxPpa::id_const(), &runner, &mut journal);

    let undo = [
        Change::FileWritten {
            path: PathBuf::from(SCX_PPA_PIN),
            existed: false,
            backup: None,
            sha256_after: String::new(),
            mode: 0o644,
            privilege: Privilege::Root,
        },
        Change::AptRepository {
            spec: SCX_PPA.to_owned(),
        },
    ];
    ScxPpa.rollback(&undo, &mut apply).expect("rolled back");

    assert!(
        runner
            .commands()
            .iter()
            .any(|command| command.contains("--remove") && command.contains(SCX_PPA)),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn verify_fails_while_scx_still_does_not_resolve() {
    let runner = ubuntu_without_scx();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let verification = ScxPpa.verify(&cx).expect("verified");
    assert_eq!(verification.failed_count(), 2);
}
