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
use gameready_core::rollback::{changes_for, execute, latest_run, plan, UndoOutcome};
use gameready_core::run::{execute as run_steps, InstallConsent, Mode};
use gameready_core::steps::{core_steps, SYSCTL_DROPIN};
use tempfile::TempDir;

const RUNTIME: &str = "/proc/sys/vm/max_map_count";
const DEFAULT: &str = "1048576\n";
const RAISED: &str = "2147483642\n";

/// Every kernel knob the fake machine answers for, with the value it starts at
/// and the value a run moves it to.
///
/// More than one step's worth on purpose: a crash sweep over a machine where
/// only one step applies proves almost nothing about the sequence.
const KNOBS: [(&str, &str, &str); 7] = [
    ("kernel.split_lock_mitigate", "1", "0"),
    ("vm.compaction_proactiveness", "20", "0"),
    ("vm.page-lock-unfairness", "5", "1"),
    ("vm.watermark_scale_factor", "10", "500"),
    ("vm.dirty_background_ratio", "10", "3"),
    ("vm.dirty_ratio", "20", "8"),
    ("vm.swappiness", "60", "180"),
];

/// Where a sysctl key is readable, the same transform the confirm pass makes.
fn proc_path(key: &str) -> String {
    format!("/proc/sys/{}", key.replace('.', "/"))
}

/// A system at the stock value, where `sysctl -w` behaves as it really does.
///
/// Modelling that side effect is what makes the apply-then-verify sequence
/// testable at all: without it, verification reads back the old value and every
/// step appears to fail.
fn system_at_default() -> MockRunner {
    let mut runner = MockRunner::new()
        .with_file(RUNTIME, DEFAULT)
        .where_command_writes(
            "sudo sysctl -w vm.max_map_count=2147483642",
            RUNTIME,
            RAISED,
        )
        .where_command_writes("sudo sysctl -w vm.max_map_count=1048576", RUNTIME, DEFAULT);

    for (key, stock, tuned) in KNOBS {
        let path = proc_path(key);
        runner = runner
            .with_file(&path, format!("{stock}\n"))
            .where_command_writes(
                format!("sudo sysctl -w {key}={tuned}"),
                &path,
                format!("{tuned}\n"),
            )
            .where_command_writes(
                format!("sudo sysctl -w {key}={stock}"),
                &path,
                format!("{stock}\n"),
            );
    }
    runner
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

/// Every file on the fake machine, for comparing before against after.
fn snapshot(runner: &MockRunner) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = runner
        .paths()
        .into_iter()
        .filter_map(|path| {
            let contents = runner.file(&path)?;
            Some((path.display().to_string(), contents))
        })
        .collect();
    files.sort();
    files
}

#[test]
fn a_run_killed_partway_still_rolls_back_to_where_it_started() {
    // Sweeps the kill across the apply's own commands. The existing per-step
    // tests stop at "enough records exist to undo this"; this one goes on to
    // undo it and check the machine, which is the claim a user cares about.
    //
    // Bounded by what a clean apply issues, because the injected failure is an
    // index into every command the runner sees. An index past the apply lands
    // in the rollback instead, which tests the opposite thing.
    let clean = TempDir::new().expect("temp dir");
    let counter = system_at_default();
    apply(&counter, &StatePaths::new(clean.path().to_path_buf()));
    let applied_commands = counter.commands().len();
    assert!(
        applied_commands >= 5,
        "the fake machine only ran {applied_commands} commands, too few to sweep"
    );

    for failure_point in 0..applied_commands {
        let dir = TempDir::new().expect("temp dir");
        let paths = StatePaths::new(dir.path().to_path_buf());
        let runner = system_at_default().failing_at(failure_point);
        let before = snapshot(&runner);

        apply(&runner, &paths);
        rollback_latest(&runner, &paths);

        assert_eq!(
            snapshot(&runner),
            before,
            "killed at command {failure_point} and the rollback did not put it back"
        );
    }
}

#[test]
fn a_journal_cut_mid_write_still_undoes_everything_written_before_the_cut() {
    // A kill during the append leaves an unterminated last line. Reading that
    // as whole-file corruption used to make every finished run in the journal
    // unrollbackable, which is a far worse outcome than losing the last record.
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let runner = system_at_default();

    apply(&runner, &paths);
    assert!(runner.file(SYSCTL_DROPIN).is_some(), "nothing was applied");

    let journal_path = paths.journal();
    let whole = std::fs::read(&journal_path).expect("reads");
    let last_line_starts = whole
        .iter()
        .enumerate()
        .filter(|(_, byte)| **byte == b'\n')
        .map(|(index, _)| index + 1)
        .nth_back(1)
        .expect("the run wrote more than one record");
    std::fs::write(&journal_path, &whole[..last_line_starts + 5]).expect("truncates");

    let outcomes = rollback_latest(&runner, &paths);

    assert!(!outcomes.is_empty(), "the torn journal read as empty");
    assert!(
        outcomes.iter().all(|outcome| !outcome.is_failure()),
        "rollback failed on a truncated journal: {outcomes:?}"
    );
    assert_eq!(runner.file(SYSCTL_DROPIN), None, "the drop-in survived");
}

#[test]
fn the_fake_machine_engages_more_than_one_step() {
    // Guards the sweep above. If seeding drifts so that only one step applies,
    // the crash sweep still passes while covering almost nothing.
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let runner = system_at_default();

    let run = apply(&runner, &paths);
    let records = load(&paths.journal()).expect("reads");
    let changes = changes_for(&records, run);

    assert!(
        runner.commands().len() >= 5,
        "only {} commands ran, too few to sweep a kill across",
        runner.commands().len()
    );
    assert!(
        changes.len() >= 5,
        "only {} changes recorded, so the sweep undoes almost nothing",
        changes.len()
    );
}
