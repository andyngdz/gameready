//! Prove a step end to end on the live machine.
//!
//! Applies, verifies, rolls back, and verifies the change is gone. Containers
//! cannot write `/proc/sys`, so this is the only place steps that touch kernel
//! state are exercised against a real kernel.

use crate::exec::CommandRunner;
use crate::improvement::{ApplyCx, CoreCx, CoreImprovement, Probe};
use crate::journal::{Change, Journal, Undo};
use crate::run::domain::{Phase, RevertCheck, SelftestResult, StepSelftest};

/// Runs the full cycle for every step, skipping the ones this machine cannot
/// take right now.
pub fn selftest(
    steps: Vec<Box<dyn CoreImprovement>>,
    cx: &CoreCx<'_>,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
) -> Vec<StepSelftest> {
    steps
        .into_iter()
        .map(|step| StepSelftest {
            step: step.id(),
            result: one(step.as_ref(), cx, runner, journal),
        })
        .collect()
}

fn one(
    step: &dyn CoreImprovement,
    cx: &CoreCx<'_>,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
) -> SelftestResult {
    match step.probe(cx) {
        // An outdated install is still work a run would do, so it cycles too.
        Ok(Probe::Applicable | Probe::UpdateAvailable { .. }) => cycle(step, cx, runner, journal),
        // Not a failure. A machine that cannot take a step has told us
        // something true about itself, not about the step.
        Ok(probe) => SelftestResult::Skipped {
            reason: probe.describe(),
        },
        Err(error) => SelftestResult::ProbeFailed {
            error: error.describe(),
        },
    }
}

fn cycle(
    step: &dyn CoreImprovement,
    cx: &CoreCx<'_>,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
) -> SelftestResult {
    let mut apply_cx = ApplyCx::new(*cx, step.id(), runner, journal);

    let applied = step.apply(&mut apply_cx);
    let recorded = apply_cx.recorded().to_vec();

    // Verification runs before the rollback but is not reported until after it.
    // The selftest changes a live machine, so every path from here has to reach
    // the rollback: returning early on a failed check would leave the change on
    // the user's system and blame the step for it.
    let verified = applied
        .as_ref()
        .err()
        .map_or_else(|| verify_failure(step, cx), |_| None);
    let undone = step.rollback(&recorded, &mut apply_cx);

    if let Err(error) = applied {
        return failed(Phase::Apply, error.describe());
    }
    if let Some(detail) = verified {
        return failed(Phase::Verify, detail);
    }
    if let Err(error) = undone {
        return failed(Phase::Rollback, error.describe());
    }

    confirm_reverted(step, cx, &recorded)
}

/// Reads the system back one last time to prove the rollback took the change
/// away.
fn confirm_reverted(
    step: &dyn CoreImprovement,
    cx: &CoreCx<'_>,
    recorded: &[Change],
) -> SelftestResult {
    // A step whose every change is a package install has nothing to read back:
    // removing a package is not the inverse of installing one, so its undo is a
    // report by design. Demanding that the change disappear would fail a step
    // that behaved exactly as documented.
    let reverts = recorded
        .iter()
        .any(|change| !matches!(change.inverse(), Undo::ReportPackages { .. }));

    if !reverts {
        return SelftestResult::Passed {
            reverted: RevertCheck::NotApplicable,
        };
    }

    // Verification must fail now: the change is supposed to be gone.
    if step
        .verify(cx)
        .is_ok_and(|verification| verification.passed())
    {
        return failed(
            Phase::Reverted,
            "verification still passes, so the rollback did not undo the change".to_owned(),
        );
    }

    SelftestResult::Passed {
        reverted: RevertCheck::Confirmed,
    }
}

/// The reason verification did not hold, or `None` when it did.
fn verify_failure(step: &dyn CoreImprovement, cx: &CoreCx<'_>) -> Option<String> {
    match step.verify(cx) {
        Ok(verification) if verification.passed() => None,
        Ok(verification) => Some(format!(
            "{} of {} checks did not pass",
            verification.failed_count(),
            verification.total_count()
        )),
        Err(error) => Some(error.describe()),
    }
}

const fn failed(phase: Phase, detail: String) -> SelftestResult {
    SelftestResult::Failed { phase, detail }
}

#[cfg(test)]
#[path = "selftest_test.rs"]
mod selftest_test;
