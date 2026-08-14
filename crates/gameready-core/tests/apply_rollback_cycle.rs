//! The whole apply-then-rollback cycle, which is where every rollback bug so
//! far actually lived.
//!
//! Each piece was unit tested and each piece was correct. The failures were all
//! in the sequence: a second apply rewriting the file, a rollback targeting the
//! previous rollback, an undo blaming the user for gameready's own rewrite.
//! Nothing that tests one call at a time catches those.

// An integration test is its own crate, so the crate-level allow in lib.rs does
// not reach here. A test reports failure by panicking either way.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use gameready_core::exec::CommandRunner;
use gameready_core::facts::{Family, SystemFacts};
use gameready_core::improvement::{CoreCx, Privilege};
use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::{load, Journal, RunId, StatePaths};
use gameready_core::rollback::{execute, latest_run, plan, UndoOutcome};
use gameready_core::run::{execute as run_steps, InstallConsent, Mode};
use gameready_core::steps::{core_steps, SYSCTL_DROPIN};
use tempfile::TempDir;

const RUNTIME: &str = "/proc/sys/vm/max_map_count";
const DEFAULT: &str = "1048576\n";
const RAISED: &str = "2147483642\n";

/// A system at the stock value, where `sysctl -w` behaves as it really does.
///
/// Modelling that side effect is what makes the apply-then-verify sequence
/// testable at all: without it, verification reads back the old value and every
/// step appears to fail.
fn system_at_default() -> MockRunner {
    MockRunner::new()
        .with_file(RUNTIME, DEFAULT)
        .where_command_writes(
            "sudo sysctl -w vm.max_map_count=2147483642",
            RUNTIME,
            RAISED,
        )
        .where_command_writes("sudo sysctl -w vm.max_map_count=1048576", RUNTIME, DEFAULT)
}

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

/// Applies every core step against the given system.
fn apply(runner: &MockRunner, paths: &StatePaths) -> RunId {
    let run = RunId::generate();
    let mut journal = Journal::open(paths.clone(), run).expect("journal opens");
    run_steps(
        core_steps(),
        &CoreCx::new(&facts(), runner),
        &mut journal,
        Mode::Apply,
        InstallConsent::Declined,
        &[],
        &mut |_| {},
    )
    .expect("run completes");
    run
}

/// Rolls back whichever run a bare `gameready rollback` would pick.
fn rollback_latest(runner: &MockRunner, paths: &StatePaths) -> Vec<UndoOutcome> {
    let records = load(&paths.journal()).expect("reads");
    let Some(target) = latest_run(&records) else {
        return Vec::new();
    };
    let undo_plan = plan(&records, target).expect("plans");
    let mut journal = Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
    let report = execute(&undo_plan, runner, &mut journal).expect("rollback runs");
    report.undos.into_iter().map(|undo| undo.outcome).collect()
}

#[test]
fn apply_then_rollback_returns_the_system_to_where_it_started() {
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let runner = system_at_default();

    apply(&runner, &paths);
    assert!(runner.file(SYSCTL_DROPIN).is_some(), "nothing was applied");

    let outcomes = rollback_latest(&runner, &paths);

    assert!(
        outcomes.iter().all(|outcome| !outcome.is_failure()),
        "rollback failed: {outcomes:?}"
    );
    assert_eq!(runner.file(SYSCTL_DROPIN), None, "the drop-in survived");
    assert!(
        runner
            .commands()
            .contains(&"sudo sysctl -w vm.max_map_count=1048576".to_owned()),
        "the runtime value was never restored: {:?}",
        runner.commands()
    );
}

#[test]
fn a_bare_rollback_after_a_rollback_does_not_target_the_rollback() {
    // The rollback journals itself under a fresh run id. Picking the newest run
    // outright made the second bare rollback report "nothing to undo" while the
    // applied run sat untouched.
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let runner = system_at_default();

    apply(&runner, &paths);
    let first = rollback_latest(&runner, &paths);
    assert!(!first.is_empty(), "the first rollback found nothing to do");

    let second = rollback_latest(&runner, &paths);
    assert!(
        second.is_empty(),
        "the second rollback re-targeted something: {second:?}"
    );
}

#[test]
fn rolling_back_an_older_run_points_at_the_run_that_owns_the_file_now() {
    // The managed header carries the run id, so two applies of the same step
    // write different bytes. Undoing the older one must say which run owns the
    // file rather than reporting a user edit.
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let runner = system_at_default();

    let older = apply(&runner, &paths);

    // A second apply, as happens when someone resets the value and re-runs.
    runner
        .write_file(RUNTIME.as_ref(), DEFAULT, Privilege::User)
        .expect("reset");
    let newer = apply(&runner, &paths);
    assert_ne!(older, newer);

    let records = load(&paths.journal()).expect("reads");
    let undo_plan = plan(&records, older).expect("plans");
    let mut journal = Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
    let report = execute(&undo_plan, &runner, &mut journal).expect("rollback runs");

    let pointed_at_owner = report.undos.iter().any(|undo| match &undo.outcome {
        UndoOutcome::Left { reason } => reason.contains(&newer.to_string()),
        UndoOutcome::Reverted { .. }
        | UndoOutcome::AlreadyGone
        | UndoOutcome::Refused { .. }
        | UndoOutcome::Failed { .. } => false,
    });
    assert!(
        pointed_at_owner,
        "the older rollback did not name the owning run: {:?}",
        report.undos
    );
    assert!(
        runner.file(SYSCTL_DROPIN).is_some(),
        "another run's file was deleted"
    );
}

#[test]
fn applying_twice_leaves_one_file_and_one_undoable_state() {
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let runner = MockRunner::new().with_file(RUNTIME, RAISED);

    apply(&runner, &paths);
    apply(&runner, &paths);

    // The value is already correct, so the second run has nothing to do and
    // records nothing that a rollback would have to reverse.
    let records = load(&paths.journal()).expect("reads");
    assert!(
        latest_run(&records).is_none(),
        "an already-correct system recorded changes"
    );
}
