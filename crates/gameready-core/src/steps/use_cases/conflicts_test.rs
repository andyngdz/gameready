use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::systemd::UnitState;

/// A systemd box where every competing daemon answers as given.
fn box_where(unit: &str, enabled: &str, active: &str) -> MockRunner {
    MockRunner::new()
        .with_binary("systemctl")
        .answering(format!("systemctl is-enabled {unit}"), enabled)
        .answering(format!("systemctl is-active {unit}"), active)
}

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Arch)
}

#[test]
fn a_machine_with_none_of_them_installed_is_clean() {
    // Unseeded is-enabled answers empty, which is what systemctl prints for a
    // unit that does not exist.
    let runner = MockRunner::new().with_binary("systemctl");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let probe = Conflicts.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }), "{probe:?}");
}

#[test]
fn a_running_daemon_is_reported_as_a_conflict() {
    let runner = box_where("tuned.service", "enabled", "active");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    match Conflicts.probe(&cx).expect("probed") {
        Probe::Conflict { with, .. } => {
            assert!(with.starts_with("tuned.service"), "{with}")
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::NotApplicable { .. }
        | Probe::Unknown { .. }) => panic!("expected Conflict, got {other:?}"),
    }
}

#[test]
fn an_installed_but_stopped_daemon_is_not_a_conflict() {
    // It is inert. Warning about it would be noise the user learns to ignore.
    let runner = box_where("tuned.service", "disabled", "inactive");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let probe = Conflicts.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }), "{probe:?}");
}

#[test]
fn a_daemon_enabled_for_the_next_boot_is_a_conflict() {
    let runner = box_where("ananicy-cpp.service", "enabled", "inactive");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    assert!(matches!(
        Conflicts.probe(&cx).expect("probed"),
        Probe::Conflict { .. }
    ));
}

#[test]
fn two_live_daemons_are_named_together() {
    let mut runner = box_where("tuned.service", "enabled", "active");
    runner = runner
        .answering("systemctl is-enabled ananicy-cpp.service", "enabled")
        .answering("systemctl is-active ananicy-cpp.service", "active");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    match Conflicts.probe(&cx).expect("probed") {
        Probe::Conflict {
            with,
            detail,
            yours,
        } => {
            assert!(with.starts_with("ananicy-cpp.service"), "{with}");
            assert!(detail.contains("tuned.service"), "{detail}");
            assert_eq!(
                yours,
                Some("systemctl disable --now ananicy-cpp.service".to_owned())
            );
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::NotApplicable { .. }
        | Probe::Unknown { .. }) => panic!("expected Conflict, got {other:?}"),
    }
}

#[test]
fn a_machine_without_systemd_reports_unknown_rather_than_clean() {
    // A container has no systemctl. Reporting a clean machine from a query that
    // never ran would be a lie the user acts on.
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let probe = Conflicts.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::Unknown { .. }), "{probe:?}");
}

#[test]
fn probing_changes_nothing() {
    let runner = box_where("tuned.service", "enabled", "active");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    Conflicts.probe(&cx).expect("probed");

    assert!(
        runner
            .commands()
            .iter()
            .all(|command| command.contains("is-enabled") || command.contains("is-active")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn the_unreachable_apply_refuses_rather_than_doing_nothing() {
    // Reached only if the executor changes to apply a reporting step, which is
    // a bug that should be loud.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = crate::journal::Journal::open(
        crate::journal::StatePaths::new(dir.path().to_path_buf()),
        crate::journal::RunId::generate(),
    )
    .expect("open");
    let mut apply = ApplyCx::new(cx, Conflicts::id_const(), &runner, &mut journal);

    assert!(Conflicts.apply(&mut apply).is_err());
    assert!(apply.recorded().is_empty());
}

#[test]
fn every_competing_daemon_is_asked_about() {
    let runner = MockRunner::new().with_binary("systemctl");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    Conflicts.probe(&cx).expect("probed");

    for daemon in COMPETING_DAEMONS {
        assert!(
            runner
                .commands()
                .contains(&format!("systemctl is-enabled {}", daemon.unit)),
            "{} was never asked about",
            daemon.unit
        );
    }
}

#[test]
fn unit_states_map_onto_the_conflict_decision() {
    assert!(UnitState::Running.is_live());
    assert!(!UnitState::Absent.is_live());
}
