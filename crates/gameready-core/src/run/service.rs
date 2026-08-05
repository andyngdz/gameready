//! Running a set of improvements.

use std::time::Instant;

use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::{
    ApplyCx, CoreCx, CoreImprovement, Outcome, Probe, RollbackStatus, SkipReason, StepError,
};
use crate::journal::{Journal, JournalEvent};
use crate::run::domain::{Mode, RunEvent, RunReport, StepReport};
use crate::run::errors::RunError;

/// Probes, plans, applies, and verifies a set of steps.
///
/// The phases are separate on purpose. Every step is probed before any step
/// applies, so the plan shown to the user is complete and a precondition that
/// fails costs nothing. In [`Mode::DryRun`] the run stops after probing.
///
/// A step whose `verify` fails is rolled back from its own recorded changes and
/// reported as failed, not applied. One step failing does not stop the rest:
/// the failure is contained to that step, which has already been undone.
pub fn execute(
    steps: Vec<Box<dyn CoreImprovement>>,
    facts: &SystemFacts,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
    mode: Mode,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<RunReport, RunError> {
    let started = Instant::now();
    let cx = CoreCx::new(facts, runner);

    let mut reports = Vec::with_capacity(steps.len());
    let mut pending = Vec::new();

    for step in steps {
        on_event(RunEvent::Probing { step: step.id() });
        match probe_outcome(step.as_ref(), &cx, mode) {
            Settled::Now(outcome) => reports.push(report(step.as_ref(), outcome)),
            Settled::Apply => pending.push(step),
        }
    }

    on_event(RunEvent::Planned {
        applicable: pending.len(),
        skipped: reports.len(),
    });

    for step in pending {
        on_event(RunEvent::Applying {
            step: step.id(),
            name: step.name().to_owned(),
        });

        journal.append(JournalEvent::StepBegin { step: step.id() })?;
        let outcome = apply_and_verify(step.as_ref(), &cx, runner, journal);

        journal.append(JournalEvent::StepEnd {
            step: step.id(),
            outcome: outcome.label().to_owned(),
        })?;
        on_event(RunEvent::Finished {
            step: step.id(),
            label: outcome.label().to_owned(),
        });

        reports.push(report(step.as_ref(), outcome));
    }

    Ok(RunReport {
        run: journal.run(),
        mode,
        steps: reports,
        installed_dependencies: Vec::new(),
        took: started.elapsed(),
    })
}

/// Whether probing settled a step's fate or left it to apply.
enum Settled {
    Now(Outcome),
    Apply,
}

fn probe_outcome(step: &dyn CoreImprovement, cx: &CoreCx<'_>, mode: Mode) -> Settled {
    match step.probe(cx) {
        Ok(Probe::Applicable) if mode.mutates() => Settled::Apply,
        Ok(Probe::Applicable) => Settled::Now(Outcome::Skipped {
            reason: SkipReason::DryRun,
        }),
        Ok(Probe::AlreadyApplied { evidence }) => {
            Settled::Now(Outcome::AlreadyApplied { evidence })
        }
        Ok(Probe::NotApplicable { reason }) => Settled::Now(Outcome::NotApplicable { reason }),
        Ok(Probe::Conflict { with, detail: _ }) => Settled::Now(Outcome::Skipped {
            reason: SkipReason::Conflict { with },
        }),
        // A step that cannot read the current state cannot restore it, so an
        // unreadable probe is never permission to apply.
        Ok(Probe::Unknown { reason }) => Settled::Now(Outcome::NotApplicable { reason }),
        Err(error) => Settled::Now(Outcome::Failed {
            error: error.to_string(),
            rolled_back: RollbackStatus::NotAttempted,
        }),
    }
}

/// Applies one step, then proves the change took effect.
///
/// A failure in either phase rolls the step back from the changes it actually
/// recorded, so a partially applied step never survives as "applied".
fn apply_and_verify(
    step: &dyn CoreImprovement,
    cx: &CoreCx<'_>,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
) -> Outcome {
    let started = Instant::now();
    let mut apply_cx = ApplyCx::new(*cx, step.id(), runner, journal);

    if let Err(error) = step.apply(&mut apply_cx) {
        let recorded = apply_cx.recorded().to_vec();
        return failed(step, &recorded, &mut apply_cx, &error.to_string());
    }

    let verification = match step.verify(cx) {
        Ok(verification) => verification,
        Err(error) => {
            let recorded = apply_cx.recorded().to_vec();
            return failed(step, &recorded, &mut apply_cx, &error.to_string());
        }
    };

    if !verification.passed() {
        let recorded = apply_cx.recorded().to_vec();
        let error = StepError::VerificationFailed {
            step: step.id(),
            failed: verification.failed_count(),
            total: verification.total_count(),
        };
        return failed(step, &recorded, &mut apply_cx, &error.to_string());
    }

    Outcome::Applied {
        changes: apply_cx.recorded().to_vec(),
        verification,
        took: started.elapsed(),
    }
}

/// Undoes what the step recorded and reports the failure.
fn failed(
    step: &dyn CoreImprovement,
    recorded: &[crate::journal::Change],
    apply_cx: &mut ApplyCx<'_, CoreCx<'_>>,
    error: &str,
) -> Outcome {
    if recorded.is_empty() {
        return Outcome::Failed {
            error: error.to_owned(),
            rolled_back: RollbackStatus::NotAttempted,
        };
    }

    let rolled_back = match step.rollback(recorded, apply_cx) {
        Ok(()) => RollbackStatus::Succeeded,
        Err(undo_error) => RollbackStatus::Failed {
            detail: undo_error.to_string(),
        },
    };

    Outcome::Failed {
        error: error.to_owned(),
        rolled_back,
    }
}

fn report(step: &dyn CoreImprovement, outcome: Outcome) -> StepReport {
    StepReport {
        step: step.id(),
        name: step.name().to_owned(),
        outcome,
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
