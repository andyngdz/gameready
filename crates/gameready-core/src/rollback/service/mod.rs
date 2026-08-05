//! Reading a run out of the journal and reversing it.

mod perform;

use crate::exec::CommandRunner;
use crate::journal::{Change, Journal, JournalEvent, JournalRecord, RunId};
use crate::rollback::domain::{
    PackagePolicy, PlannedUndo, RollbackPlan, RollbackReport, UndoReport,
};
use crate::rollback::errors::RollbackError;
use crate::rollback::service::perform::perform;

/// Builds the undo sequence for one run.
///
/// Reverse `seq` order, because a step's later change may depend on its earlier
/// one: the runtime sysctl goes back before the file that persists it is
/// removed, so an interrupted rollback never leaves a file claiming a value the
/// kernel no longer has.
///
/// Touches nothing, so `--dry-run` shows exactly what would happen.
pub fn plan(records: &[JournalRecord], target: RunId) -> Result<RollbackPlan, RollbackError> {
    let mut undos: Vec<PlannedUndo> = records
        .iter()
        .filter(|record| record.run == target)
        .filter_map(|record| match &record.event {
            JournalEvent::Changed { step, change } => Some(PlannedUndo {
                step: step.clone(),
                seq: record.seq,
                undo: change.inverse(),
            }),
            JournalEvent::RunBegin { .. }
            | JournalEvent::StepBegin { .. }
            | JournalEvent::StepEnd { .. }
            | JournalEvent::RunEnd { .. }
            | JournalEvent::RollbackBegin { .. }
            | JournalEvent::Undone { .. }
            | JournalEvent::RollbackEnd { .. } => None,
        })
        .collect();

    if undos.is_empty() && !records.iter().any(|record| record.run == target) {
        return Err(RollbackError::UnknownRun {
            run: target.to_string(),
        });
    }

    undos.sort_by_key(|planned| std::cmp::Reverse(planned.seq));
    Ok(RollbackPlan { run: target, undos })
}

/// The most recent run a bare `rollback` should undo.
///
/// Not simply the newest run in the journal. A rollback records itself under a
/// fresh run id, so the newest run is often the previous rollback, which by
/// definition has nothing to undo. Runs already undone are skipped too, so
/// running `rollback` twice does not re-target work that is already reversed.
#[must_use]
pub fn latest_run(records: &[JournalRecord]) -> Option<RunId> {
    let undone: Vec<RunId> = records
        .iter()
        .filter_map(|record| match &record.event {
            JournalEvent::RollbackBegin { target } => Some(*target),
            JournalEvent::RunBegin { .. }
            | JournalEvent::StepBegin { .. }
            | JournalEvent::Changed { .. }
            | JournalEvent::StepEnd { .. }
            | JournalEvent::RunEnd { .. }
            | JournalEvent::Undone { .. }
            | JournalEvent::RollbackEnd { .. } => None,
        })
        .collect();

    records
        .iter()
        .filter(|record| matches!(record.event, JournalEvent::Changed { .. }))
        .map(|record| record.run)
        .filter(|run| !undone.contains(run))
        .max()
}

/// Performs a rollback, journalling it so a failed undo is inspectable.
pub fn execute(
    plan: &RollbackPlan,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
    packages: PackagePolicy,
) -> Result<RollbackReport, RollbackError> {
    journal.append(JournalEvent::RollbackBegin { target: plan.run })?;

    let mut undos = Vec::with_capacity(plan.undos.len());
    for planned in &plan.undos {
        let outcome = perform(&planned.undo, runner, packages);
        journal.append(JournalEvent::Undone {
            step: planned.step.clone(),
            detail: outcome.describe(),
        })?;
        undos.push(UndoReport {
            step: planned.step.clone(),
            undo: planned.undo.clone(),
            outcome,
        });
    }

    let report = RollbackReport {
        run: plan.run,
        undos,
    };
    journal.append(JournalEvent::RollbackEnd {
        undone: report.reverted(),
        failed: report.failed(),
    })?;

    Ok(report)
}

/// Every change a run recorded, newest first. Used by `status`.
#[must_use]
pub fn changes_for(records: &[JournalRecord], run: RunId) -> Vec<Change> {
    records
        .iter()
        .filter(|record| record.run == run)
        .filter_map(|record| match &record.event {
            JournalEvent::Changed { change, .. } => Some(change.clone()),
            JournalEvent::RunBegin { .. }
            | JournalEvent::StepBegin { .. }
            | JournalEvent::StepEnd { .. }
            | JournalEvent::RunEnd { .. }
            | JournalEvent::RollbackBegin { .. }
            | JournalEvent::Undone { .. }
            | JournalEvent::RollbackEnd { .. } => None,
        })
        .collect()
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod mod_test;
