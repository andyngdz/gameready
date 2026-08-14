use crate::improvement::Privilege;
use tempfile::TempDir;

use crate::improvement::ImprovementId;
use crate::infra::exec::MockRunner;
use crate::journal::{digest, Change, Journal, JournalEvent, StatePaths, Undo};

use super::*;

const DROPIN: &str = "/etc/sysctl.d/99-gameready.conf";
const WROTE: &str = "vm.max_map_count = 2147483642\n";

fn step() -> ImprovementId {
    ImprovementId::from_static("core.sysctl.max-map-count")
}

/// Writes a run that created a file and then set a sysctl, the real order.
fn recorded_run(dir: &TempDir) -> (RunId, Vec<crate::journal::JournalRecord>) {
    let paths = StatePaths::new(dir.path().to_path_buf());
    let run = RunId::generate();
    let mut journal = Journal::open(paths.clone(), run).expect("journal opens");

    journal
        .append(JournalEvent::Changed {
            step: step(),
            change: Change::FileWritten {
                path: DROPIN.into(),
                sha256_after: digest(WROTE),
                mode: 0o644,
                privilege: Privilege::Root,
            },
        })
        .expect("appends");
    journal
        .append(JournalEvent::Changed {
            step: step(),
            change: Change::SysctlRuntime {
                key: "vm.max_map_count".to_owned(),
                previous: "1048576".to_owned(),
            },
        })
        .expect("appends");

    let records = crate::journal::load(&paths.journal()).expect("reads");
    (run, records)
}

#[test]
fn undo_runs_in_reverse_order_of_the_original_changes() {
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);

    let undo_plan = plan(&records, run).expect("plans");

    // The sysctl was set second, so it goes back first: an interrupted
    // rollback must never leave a file claiming a value the kernel lost.
    assert_eq!(undo_plan.undos.len(), 2);
    assert!(matches!(undo_plan.undos[0].undo, Undo::SetSysctl { .. }));
    assert!(matches!(undo_plan.undos[1].undo, Undo::DeleteFile { .. }));
}

#[test]
fn planning_touches_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);
    let runner = MockRunner::new().with_file(DROPIN, WROTE);

    let _ = plan(&records, run).expect("plans");

    assert!(runner.commands().is_empty());
    assert!(runner.file(DROPIN).is_some());
}

#[test]
fn executing_reverses_both_changes() {
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);
    let undo_plan = plan(&records, run).expect("plans");

    let runner = MockRunner::new().with_file(DROPIN, WROTE);
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let report = execute(&undo_plan, &runner, &mut journal).expect("rollback runs");

    assert_eq!(report.reverted(), 2);
    assert_eq!(report.failed(), 0);
    assert!(runner.file(DROPIN).is_none());
    assert!(runner
        .commands()
        .contains(&"sudo sysctl -w vm.max_map_count=1048576".to_owned()));
}

#[test]
fn an_unknown_run_is_rejected() {
    let dir = TempDir::new().expect("temp dir");
    let (_, records) = recorded_run(&dir);
    assert!(plan(&records, RunId::generate()).is_err());
}

#[test]
fn the_latest_run_is_the_one_a_bare_rollback_targets() {
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);
    assert_eq!(latest_run(&records), Some(run));
}

#[test]
fn a_rollbacks_own_run_is_never_the_next_target() {
    // A rollback journals itself under a fresh run id. Picking the newest run
    // outright makes the next bare rollback target that rollback, which has
    // nothing to undo, and the real run is never reached.
    let dir = TempDir::new().expect("temp dir");
    let (applied, _) = recorded_run(&dir);
    let paths = StatePaths::new(dir.path().to_path_buf());

    let mut rollback_journal =
        Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
    rollback_journal
        .append(JournalEvent::RollbackBegin { target: applied })
        .expect("appends");
    rollback_journal
        .append(JournalEvent::RollbackEnd {
            undone: 2,
            failed: 0,
        })
        .expect("appends");

    let records = crate::journal::load(&paths.journal()).expect("reads");

    // The applied run was undone, and the rollback run holds no changes, so
    // there is nothing left to target.
    assert_eq!(latest_run(&records), None);
}

#[test]
fn a_run_that_failed_to_roll_back_is_still_targetable() {
    // The 15:25 failure on a cold sudo cache left RollbackBegin without any
    // change of its own; the applied run must stay reachable so a retry works.
    let dir = TempDir::new().expect("temp dir");
    let (applied, _) = recorded_run(&dir);
    let paths = StatePaths::new(dir.path().to_path_buf());

    let mut rollback_journal =
        Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
    rollback_journal
        .append(JournalEvent::RollbackBegin { target: applied })
        .expect("appends");

    let records = crate::journal::load(&paths.journal()).expect("reads");
    assert_eq!(latest_run(&records), None, "marked undone once begun");
    // Retrying by id always works, which is what the summary tells the user.
    assert!(plan(&records, applied).is_ok());
}

#[test]
fn rollback_is_safe_to_run_twice() {
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);
    let undo_plan = plan(&records, run).expect("plans");
    let runner = MockRunner::new().with_file(DROPIN, WROTE);
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let _ = execute(&undo_plan, &runner, &mut journal).expect("first");
    let second = execute(&undo_plan, &runner, &mut journal).expect("second");

    // The file is already gone, which is not a failure.
    assert_eq!(second.failed(), 0);
}

#[test]
fn a_command_that_exits_zero_without_changing_anything_is_reported_as_failed() {
    // The reason rollback reads the system back. `sysctl -w` exits zero inside
    // a container whose /proc is masked, and the value never moves. Trusting
    // the exit code would tell the user their machine was back to normal.
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);
    let undo_plan = plan(&records, run).expect("plans");
    let runner = MockRunner::new()
        .with_file(DROPIN, WROTE)
        .with_file("/proc/sys/vm/max_map_count", "2147483642\n");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let report = execute(&undo_plan, &runner, &mut journal).expect("rollback runs");

    assert_eq!(report.failed(), 1, "{:?}", report.undos);
    let sysctl = report
        .undos
        .iter()
        .find(|undo| matches!(undo.undo, Undo::SetSysctl { .. }))
        .expect("the sysctl undo is in the report");
    match &sysctl.outcome {
        UndoOutcome::Failed { error } => {
            assert!(error.contains("2147483642"), "{error}");
            assert!(error.contains("1048576"), "{error}");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_command_that_really_changed_the_system_still_reads_as_reverted() {
    // The same plan, with /proc answering what the undo asked for.
    let dir = TempDir::new().expect("temp dir");
    let (run, records) = recorded_run(&dir);
    let undo_plan = plan(&records, run).expect("plans");
    let runner = MockRunner::new()
        .with_file(DROPIN, WROTE)
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let report = execute(&undo_plan, &runner, &mut journal).expect("rollback runs");

    assert_eq!(report.failed(), 0, "{:?}", report.undos);
    assert_eq!(report.reverted(), 2);
}
