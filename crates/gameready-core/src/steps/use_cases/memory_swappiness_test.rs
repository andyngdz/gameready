use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::MANAGED_HEADER;

const SWAPPINESS: &str = "/proc/sys/vm/swappiness";
const DEFAULT_SWAPPINESS: &str = "60";

const ZRAM_SWAPS: &str = indoc! {"
    Filename Type Size Used Priority
    /dev/zram0 partition 8388604 0 100
"};

const DISK_SWAPS: &str = indoc! {"
    Filename Type Size Used Priority
    /swap.img file 8388604 53956 -1
"};

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

/// A machine whose primary swap is zram, sitting at the given swappiness.
fn zram_at(swappiness: &str) -> MockRunner {
    MockRunner::new()
        .with_file("/proc/swaps", ZRAM_SWAPS)
        .with_file(SWAPPINESS, format!("{swappiness}\n"))
}

/// A machine whose only swap is a disk file, this session's real machine.
fn disk_system() -> MockRunner {
    MockRunner::new()
        .with_file("/proc/swaps", DISK_SWAPS)
        .with_file(SWAPPINESS, format!("{DEFAULT_SWAPPINESS}\n"))
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
        Swappiness::id_const(),
        runner,
        &mut log,
    );
    let outcome = Swappiness.apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn disk_swap_is_not_applicable() {
    // The whole point of the step: on the common disk-swap machine it does
    // nothing, and says why.
    let runner = disk_system();
    let facts = facts();
    match Swappiness
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::NotApplicable { reason } => assert!(reason.contains("disk"), "{reason}"),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn zram_with_a_low_value_applies() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let facts = facts();
    let probe = Swappiness
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes");
    assert_eq!(probe, Probe::Applicable);
}

#[test]
fn zram_already_at_the_target_is_left_alone() {
    let runner = zram_at("180");
    let facts = facts();
    match Swappiness
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::AlreadyApplied { evidence } => assert!(evidence.contains("180")),
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn a_higher_existing_value_is_left_alone() {
    // Someone who already set this above our target does not get lowered.
    let runner = zram_at("200");
    let facts = facts();
    let probe = Swappiness
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }));
}

#[test]
fn the_plan_names_both_the_old_and_new_value() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let facts = facts();
    let plan = Swappiness
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(plan.summary.contains(DEFAULT_SWAPPINESS));
    assert!(plan.summary.contains("180"));
    assert_eq!(plan.actions.len(), 2);
}

#[test]
fn planning_changes_nothing() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let facts = facts();
    let _ = Swappiness
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(runner.commands().is_empty(), "plan ran a command");
    assert_eq!(runner.file(SWAPPINESS_DROPIN), None, "plan wrote a file");
}

#[test]
fn apply_persists_before_it_sets_the_runtime_value() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    assert!(matches!(recorded[0], Change::FileWritten { .. }));
    assert!(matches!(recorded[1], Change::SysctlRuntime { .. }));
}

#[test]
fn apply_records_the_previous_value_so_rollback_can_restore_it() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    match &recorded[1] {
        Change::SysctlRuntime { previous, .. } => assert_eq!(previous, DEFAULT_SWAPPINESS),
        other @ (Change::FileWritten { .. }
        | Change::FileRemoved { .. }
        | Change::SysfsWrite { .. }
        | Change::PackagesInstalled { .. }
        | Change::SystemdUnit { .. }
        | Change::AptRepository { .. }
        | Change::ScxScheduler { .. }
        | Change::DirCreated { .. }
        | Change::DirTreeInstalled { .. }) => panic!("expected a sysctl record, got {other:?}"),
    }
}

#[test]
fn the_file_it_writes_carries_the_marker_and_the_value() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(SWAPPINESS_DROPIN).expect("drop-in written");
    assert!(written.contains(MANAGED_HEADER));
    assert!(written.contains("step=core.memory.swappiness"));
    assert!(written.contains("vm.swappiness = 180"));
}

#[test]
fn verify_fails_when_the_change_did_not_take() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let facts = facts();
    let verification = Swappiness
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn verify_passes_only_when_both_the_runtime_and_the_file_are_right() {
    let runner = zram_at("180").with_file(SWAPPINESS_DROPIN, "vm.swappiness = 180");
    let facts = facts();
    let verification = Swappiness
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(verification.passed());
    assert_eq!(verification.total_count(), 2);
}

#[test]
fn apply_then_rollback_restores_the_prior_state() {
    let runner = zram_at(DEFAULT_SWAPPINESS);
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            Swappiness::id_const(),
            &runner,
            &mut log,
        );
        Swappiness.apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };

    assert!(
        runner.file(SWAPPINESS_DROPIN).is_some(),
        "drop-in was written"
    );

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        Swappiness::id_const(),
        &runner,
        &mut log,
    );
    Swappiness.rollback(&recorded, &mut cx).expect("rolls back");

    assert_eq!(runner.file(SWAPPINESS_DROPIN), None);
    assert!(
        runner
            .commands()
            .iter()
            .any(|cmd| cmd == "sudo sysctl -w vm.swappiness=60"),
        "rollback did not restore the previous value: {:?}",
        runner.commands()
    );
}

#[test]
fn apply_failing_midway_still_leaves_an_undoable_record() {
    for failure_point in 0..4 {
        let runner = zram_at(DEFAULT_SWAPPINESS).failing_at(failure_point);
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
