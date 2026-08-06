use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::MANAGED_HEADER;

const DEFAULT_ON_THIS_MACHINE: &str = "1048576";
const RUNTIME: &str = "/proc/sys/vm/max_map_count";

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

fn system_at(value: &str) -> MockRunner {
    MockRunner::new().with_file(RUNTIME, format!("{value}\n"))
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens")
}

#[test]
fn the_managed_header_carries_the_run_id_not_the_step_id() {
    // doctor ties a leftover file back to the run that made it. Stamping the
    // step id in both fields makes that impossible, which is what shipped
    // first.
    let dir = TempDir::new().expect("temp dir");
    let run = RunId::generate();
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let facts = facts();
    let mut log =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), run).expect("journal opens");

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        MaxMapCount::id_const(),
        &runner,
        &mut log,
    );
    MaxMapCount.apply(&mut cx).expect("applies");

    let written = runner.file(SYSCTL_DROPIN).expect("drop-in written");
    assert!(
        written.contains(&format!("run={run}")),
        "header does not name the run: {written}"
    );
    assert!(written.contains("step=core.sysctl.max-map-count"));
}

/// Runs apply against a system, returning the recorded changes.
fn apply_against(runner: &MockRunner) -> (Vec<Change>, Result<(), StepError>) {
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, runner),
        MaxMapCount::id_const(),
        runner,
        &mut log,
    );
    let outcome = MaxMapCount.apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn probe_when_absent_says_it_applies() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let facts = facts();
    let probe = MaxMapCount
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes");
    assert_eq!(probe, Probe::Applicable);
}

#[test]
fn probe_when_already_applied_says_so_with_evidence() {
    let runner = system_at("2147483642");
    let facts = facts();
    match MaxMapCount
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::AlreadyApplied { evidence } => assert!(evidence.contains("2147483642")),
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn a_higher_existing_value_is_left_alone() {
    // Someone who already set this higher than we would does not get lowered.
    let runner = system_at("4294967295");
    let facts = facts();
    let probe = MaxMapCount
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }));
}

#[test]
fn probe_fails_loudly_when_the_kernel_value_is_unreadable() {
    // A step that cannot read the current state cannot restore it, so this
    // must not fall through to Applicable.
    let runner = MockRunner::new();
    let facts = facts();
    assert!(MaxMapCount.probe(&CoreCx::new(&facts, &runner)).is_err());
}

#[test]
fn the_plan_names_both_the_old_and_new_value() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let facts = facts();
    let plan = MaxMapCount
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(plan.summary.contains(DEFAULT_ON_THIS_MACHINE));
    assert!(plan.summary.contains("2147483642"));
    assert_eq!(plan.actions.len(), 2);
}

#[test]
fn planning_changes_nothing() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let facts = facts();
    let _ = MaxMapCount
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(runner.commands().is_empty(), "plan ran a command");
    assert_eq!(runner.file(SYSCTL_DROPIN), None, "plan wrote a file");
}

#[test]
fn apply_persists_before_it_sets_the_runtime_value() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    // Persistence first: dying between the two leaves the machine correct on
    // next boot rather than correct now and wrong later.
    assert!(matches!(recorded[0], Change::FileWritten { .. }));
    assert!(matches!(recorded[1], Change::SysctlRuntime { .. }));
}

#[test]
fn apply_records_the_previous_value_so_rollback_can_restore_it() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    match &recorded[1] {
        Change::SysctlRuntime { previous, .. } => assert_eq!(previous, DEFAULT_ON_THIS_MACHINE),
        other @ (Change::FileWritten { .. }
        | Change::FileRemoved { .. }
        | Change::SysfsWrite { .. }
        | Change::PackagesInstalled { .. }
        | Change::SystemdUnit { .. }
        | Change::DirCreated { .. }
        | Change::DirTreeInstalled { .. }) => panic!("expected a sysctl record, got {other:?}"),
    }
}

#[test]
fn the_file_it_writes_carries_the_marker_doctor_looks_for() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(SYSCTL_DROPIN).expect("drop-in written");
    // Without this marker, a user who deleted their journal has no way to find
    // what gameready left behind.
    assert!(written.contains(MANAGED_HEADER));
    assert!(written.contains("vm.max_map_count = 2147483642"));
}

#[test]
fn verify_fails_when_the_change_did_not_take() {
    // The runtime value never moved, so verification must not pass.
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let facts = facts();
    let verification = MaxMapCount
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn verify_passes_only_when_both_the_runtime_and_the_file_are_right() {
    let runner = system_at("2147483642").with_file(SYSCTL_DROPIN, "vm.max_map_count = 2147483642");
    let facts = facts();
    let verification = MaxMapCount
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(verification.passed());
    assert_eq!(verification.total_count(), 2);
}

#[test]
fn a_correct_runtime_value_without_the_file_still_fails_verification() {
    // The change would not survive a reboot, so it is not done.
    let runner = system_at("2147483642");
    let facts = facts();
    let verification = MaxMapCount
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn apply_then_rollback_restores_the_prior_state() {
    let runner = system_at(DEFAULT_ON_THIS_MACHINE);
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            MaxMapCount::id_const(),
            &runner,
            &mut log,
        );
        MaxMapCount.apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };

    assert!(runner.file(SYSCTL_DROPIN).is_some(), "drop-in was written");

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        MaxMapCount::id_const(),
        &runner,
        &mut log,
    );
    MaxMapCount
        .rollback(&recorded, &mut cx)
        .expect("rolls back");

    // The file is gone and the restoring sysctl call was issued with the value
    // the machine had before the run.
    assert_eq!(runner.file(SYSCTL_DROPIN), None);
    assert!(
        runner
            .commands()
            .iter()
            .any(|cmd| cmd == "sudo sysctl -w vm.max_map_count=1048576"),
        "rollback did not restore the previous value: {:?}",
        runner.commands()
    );
}

#[test]
fn apply_failing_midway_still_leaves_an_undoable_record() {
    // Sweeps the failure point across the whole command sequence. At every
    // position, whatever was already done must be described by the records the
    // step recorded, or that part of the change is unrecoverable.
    for failure_point in 0..4 {
        let runner = system_at(DEFAULT_ON_THIS_MACHINE).failing_at(failure_point);
        let (recorded, outcome) = apply_against(&runner);

        if outcome.is_ok() {
            continue;
        }

        let mutating_commands = runner
            .commands()
            .iter()
            .filter(|cmd| cmd.starts_with("sudo"))
            .count();
        assert!(
            recorded.len() >= mutating_commands,
            "failure at {failure_point} ran {mutating_commands} mutating commands \
             but recorded only {} undo records",
            recorded.len()
        );
    }
}
