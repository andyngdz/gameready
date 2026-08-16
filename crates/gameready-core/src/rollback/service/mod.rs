//! Reading a run out of the journal and reversing it.

mod confirm;
mod perform;
mod perform_files;
mod perform_steam;

use crate::exec::CommandRunner;
use crate::journal::{Change, Journal, JournalEvent, JournalRecord, RunId, Undo};
use crate::rollback::domain::{PlannedUndo, RollbackPlan, RollbackReport, UndoOutcome, UndoReport};
use crate::rollback::errors::RollbackError;
use crate::rollback::service::confirm::confirm;
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
///
/// A run is only counted as undone when the rollback that targeted it wrote its
/// `RollbackEnd`. One that was killed or failed partway has `RollbackBegin`
/// with no `RollbackEnd`, and its target is only partially undone: skipping it
/// would silently leave the rest of its changes in place and move on to an
/// older run. Re-targeting it instead is safe, because each undo is idempotent.
#[must_use]
pub fn latest_run(records: &[JournalRecord]) -> Option<RunId> {
    let completed: Vec<RunId> = records
        .iter()
        .filter_map(|record| match &record.event {
            JournalEvent::RollbackEnd { .. } => Some(record.run),
            JournalEvent::RunBegin { .. }
            | JournalEvent::StepBegin { .. }
            | JournalEvent::Changed { .. }
            | JournalEvent::StepEnd { .. }
            | JournalEvent::RunEnd { .. }
            | JournalEvent::RollbackBegin { .. }
            | JournalEvent::Undone { .. } => None,
        })
        .collect();

    let undone: Vec<RunId> = records
        .iter()
        .filter_map(|record| match &record.event {
            JournalEvent::RollbackBegin { target } if completed.contains(&record.run) => {
                Some(*target)
            }
            JournalEvent::RunBegin { .. }
            | JournalEvent::StepBegin { .. }
            | JournalEvent::Changed { .. }
            | JournalEvent::StepEnd { .. }
            | JournalEvent::RunEnd { .. }
            | JournalEvent::RollbackBegin { .. }
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
) -> Result<RollbackReport, RollbackError> {
    journal.append(JournalEvent::RollbackBegin { target: plan.run })?;

    let mut undos = Vec::with_capacity(plan.undos.len());
    for planned in &plan.undos {
        let outcome = confirmed(perform(&planned.undo, runner), &planned.undo, runner);
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

/// Downgrades a claimed revert the system does not actually show.
///
/// An undo reports success from the exit code of whatever it ran, and an exit
/// code is the tool's opinion, not the machine's state. Telling a user their
/// system is back to normal is the one claim this must not get wrong, so the
/// claim is checked before it is made.
///
/// Only a `Reverted` is checked. Every other outcome already says the system
/// was not changed, and reading it back would answer a question nobody asked.
fn confirmed(outcome: UndoOutcome, undo: &Undo, runner: &dyn CommandRunner) -> UndoOutcome {
    if !matches!(outcome, UndoOutcome::Reverted { .. }) {
        return outcome;
    }
    match confirm(undo, runner) {
        None => outcome,
        Some(reason) => UndoOutcome::Failed {
            error: format!("reported as undone, but {reason}"),
        },
    }
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
