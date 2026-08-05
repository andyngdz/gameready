//! Applying and verifying a single step.

use std::time::Instant;

use crate::exec::CommandRunner;
use crate::improvement::{ApplyCx, CoreCx, CoreImprovement, Outcome, RollbackStatus, StepError};
use crate::journal::{Change, Journal};

/// Applies one step, then proves the change took effect.
///
/// A failure in either phase rolls the step back from the changes it actually
/// recorded, so a partially applied step never survives as "applied".
pub fn apply_and_verify(
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

fn failed(
    step: &dyn CoreImprovement,
    recorded: &[Change],
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

#[cfg(test)]
#[path = "apply_step_test.rs"]
mod apply_step_test;
