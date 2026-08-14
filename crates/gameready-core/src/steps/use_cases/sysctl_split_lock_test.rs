use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::MANAGED_HEADER;

/// What the kernel ships, and what this machine reported on 2026-08-09.
const KERNEL_DEFAULT: &str = "1";
const RUNTIME: &str = "/proc/sys/kernel/split_lock_mitigate";

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

/// Runs apply against a system, returning the recorded changes.
fn apply_against(runner: &MockRunner) -> (Vec<Change>, Result<(), StepError>) {
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, runner),
        SplitLock::id_const(),
        runner,
        &mut log,
    );
    let outcome = SplitLock.apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn probe_on_a_kernel_still_punishing_split_locks_says_it_applies() {
    let runner = system_at(KERNEL_DEFAULT);
    let facts = facts();
    let probe = SplitLock
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes");
    assert_eq!(probe, Probe::Applicable);
}

#[test]
fn probe_when_already_off_says_so_with_evidence() {
    let runner = system_at("0");
    let facts = facts();
    match SplitLock
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::AlreadyApplied { evidence } => {
            assert!(
                evidence.contains("kernel.split_lock_mitigate"),
                "{evidence}"
            );
            assert!(evidence.contains('0'), "{evidence}");
        }
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn it_stands_down_when_gamemode_is_here_to_do_the_same_thing() {
    // gamemode ships disable_splitlock=1, so it clears this while a client runs
    // and restores it after. Applying permanently on top would be a change the
    // user did not need.
    let runner = system_at(KERNEL_DEFAULT).with_binary("gamemoded");
    let facts = facts();
    match SplitLock
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::AlreadyApplied { evidence } => assert!(evidence.contains("gamemode"), "{evidence}"),
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn without_gamemode_it_still_applies_for_games_not_started_through_gamemoderun() {
    let runner = system_at(KERNEL_DEFAULT);
    let facts = facts();
    assert_eq!(
        SplitLock
            .probe(&CoreCx::new(&facts, &runner))
            .expect("probes"),
        Probe::Applicable
    );
}

#[test]
fn a_kernel_without_the_detector_is_not_applicable_rather_than_an_error() {
    // No file at all: not x86, or a kernel built without split-lock detection.
    // There is nothing to turn off, and that is not a failure.
    let runner = MockRunner::new();
    let facts = facts();
    match SplitLock
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::NotApplicable { reason } => assert!(reason.contains("split-lock"), "{reason}"),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn a_value_the_kernel_cannot_have_fails_loudly() {
    // A step that cannot read the current state cannot restore it, so an
    // unparseable value must not fall through to Applicable.
    let runner = system_at("sometimes");
    let facts = facts();
    assert!(SplitLock.probe(&CoreCx::new(&facts, &runner)).is_err());
}

#[test]
fn the_plan_names_both_the_old_and_new_value() {
    let runner = system_at(KERNEL_DEFAULT);
    let facts = facts();
    let plan = SplitLock
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(plan.summary.contains("1 -> 0"), "{}", plan.summary);
    assert_eq!(plan.actions.len(), 2);
}

#[test]
fn planning_changes_nothing() {
    let runner = system_at(KERNEL_DEFAULT);
    let facts = facts();
    let _ = SplitLock
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(runner.commands().is_empty(), "plan ran a command");
    assert_eq!(runner.file(SPLIT_LOCK_DROPIN), None, "plan wrote a file");
}

#[test]
fn apply_persists_before_it_sets_the_runtime_value() {
    let runner = system_at(KERNEL_DEFAULT);
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    assert!(matches!(recorded[0], Change::FileWritten { .. }));
    assert!(matches!(recorded[1], Change::SysctlRuntime { .. }));
}

#[test]
fn apply_records_the_previous_value_so_rollback_can_restore_it() {
    let runner = system_at(KERNEL_DEFAULT);
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    match &recorded[1] {
        Change::SysctlRuntime { key, previous } => {
            assert_eq!(key, "kernel.split_lock_mitigate");
            assert_eq!(previous, KERNEL_DEFAULT);
        }
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
    let runner = system_at(KERNEL_DEFAULT);
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(SPLIT_LOCK_DROPIN).expect("drop-in written");
    assert!(written.contains(MANAGED_HEADER));
    assert!(written.contains("kernel.split_lock_mitigate = 0"));
    assert!(written.contains("step=core.sysctl.split-lock"));
}

#[test]
fn its_dropin_is_not_the_one_another_sysctl_step_owns() {
    // One file per step. Sharing one would make each step's rollback delete the
    // other's setting.
    use crate::steps::constants::{SWAPPINESS_DROPIN, SYSCTL_DROPIN};
    assert_ne!(SPLIT_LOCK_DROPIN, SYSCTL_DROPIN);
    assert_ne!(SPLIT_LOCK_DROPIN, SWAPPINESS_DROPIN);
}

#[test]
fn verify_fails_when_the_change_did_not_take() {
    let runner = system_at(KERNEL_DEFAULT);
    let facts = facts();
    let verification = SplitLock
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn verify_passes_only_when_both_the_runtime_and_the_file_are_right() {
    let runner = system_at("0").with_file(SPLIT_LOCK_DROPIN, "kernel.split_lock_mitigate = 0");
    let facts = facts();
    let verification = SplitLock
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(verification.passed());
    assert_eq!(verification.total_count(), 2);
}

#[test]
fn a_correct_runtime_value_without_the_file_still_fails_verification() {
    // The change would not survive a reboot, so it is not done.
    let runner = system_at("0");
    let facts = facts();
    let verification = SplitLock
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn apply_then_rollback_restores_the_prior_state() {
    let runner = system_at(KERNEL_DEFAULT);
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            SplitLock::id_const(),
            &runner,
            &mut log,
        );
        SplitLock.apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };

    assert!(runner.file(SPLIT_LOCK_DROPIN).is_some(), "drop-in written");

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        SplitLock::id_const(),
        &runner,
        &mut log,
    );
    SplitLock.rollback(&recorded, &mut cx).expect("rolls back");

    assert_eq!(runner.file(SPLIT_LOCK_DROPIN), None);
    assert!(
        runner
            .commands()
            .iter()
            .any(|cmd| cmd == "sudo sysctl -w kernel.split_lock_mitigate=1"),
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
        let runner = system_at(KERNEL_DEFAULT).failing_at(failure_point);
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
