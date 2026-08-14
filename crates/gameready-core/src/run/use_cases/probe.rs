//! Sorting every step into what the run will do with it, by reading only.
//!
//! Three answers, not two. A step whose probe said no can still be held open
//! when the run contains something that would change the answer, and that
//! decision has to be made here, before the user is asked anything, so a step
//! promoted later cannot fetch a package nobody agreed to.

use crate::improvement::{
    CoreCx, CoreImprovement, ImprovementId, Outcome, Probe, RollbackStatus, SkipReason,
};
use crate::run::domain::{Contested, Deferred, RunEvent, StepReport};

/// The three lists probing sorts every step into.
pub(super) struct Probed {
    pub(super) settled: Vec<StepReport>,
    pub(super) pending: Vec<Box<dyn CoreImprovement>>,
    pub(super) deferred: Vec<Deferred>,
    pub(super) contested: Vec<Contested>,
}

/// A step the probe ruled out that names something, waiting to find out
/// whether the run actually contains what it named.
struct Candidate {
    step: Box<dyn CoreImprovement>,
    outcome: Outcome,
}

/// Probes every step once and sorts the results.
pub(super) fn probe_all(
    steps: Vec<Box<dyn CoreImprovement>>,
    cx: &CoreCx<'_>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Probed {
    let total = steps.len();
    let mut settled = Vec::with_capacity(total);
    let mut pending = Vec::new();
    let mut candidates = Vec::new();
    let mut contested = Vec::new();

    for (index, step) in steps.into_iter().enumerate() {
        on_event(RunEvent::Probing {
            step: step.id(),
            done: index + 1,
            total,
        });
        match probe_outcome(step.as_ref(), cx) {
            Settled::Apply => pending.push(step),
            Settled::Contested { with, detail } => contested.push(Contested { step, with, detail }),
            Settled::Now(outcome) if may_reopen(step.as_ref(), &outcome) => {
                candidates.push(Candidate { step, outcome });
            }
            Settled::Now(outcome) => finish(step.as_ref(), outcome, &mut settled, on_event),
        }
    }

    let deferred = hold_open(candidates, &pending, &mut settled, on_event);

    on_event(RunEvent::Planned {
        applicable: pending.len() + deferred.len(),
        skipped: settled.len(),
    });

    Probed {
        settled,
        pending,
        deferred,
        contested,
    }
}

/// Records a step as settled and tells the caller it finished.
pub(crate) fn finish(
    step: &dyn CoreImprovement,
    outcome: Outcome,
    settled: &mut Vec<StepReport>,
    on_event: &mut dyn FnMut(RunEvent),
) {
    on_event(RunEvent::Finished {
        step: step.id(),
        name: step.name().to_owned(),
        kind: outcome.kind(),
        detail: outcome.detail(),
    });
    settled.push(StepReport::for_step(step, outcome));
}

/// The skip a conflict settles as when it cannot be asked about.
///
/// Used by the sweep, which meets a conflict mid-run where the takeover
/// question has already been asked and cannot be re-opened. A machine that
/// changed since is reported, not re-decided.
#[must_use]
pub(super) fn contested_skip(with: &str, detail: &str) -> Outcome {
    Outcome::Skipped {
        reason: SkipReason::Conflict {
            with: with.to_owned(),
            detail: detail.to_owned(),
            yours: None,
        },
    }
}

/// Whether a ruled-out step could answer differently later in this run.
///
/// A step that could not tell has as much reason for a second look as one that
/// read a clear no, so both endings are held open. A conflict is not: it is
/// held for the takeover question when the run can clear it, and settled
/// otherwise; either way nothing in this run is going to change its answer.
fn may_reopen(step: &dyn CoreImprovement, outcome: &Outcome) -> bool {
    let reopenable = matches!(
        outcome,
        Outcome::NotApplicable { .. }
            | Outcome::Skipped {
                reason: SkipReason::CouldNotTell { .. }
            }
    );
    !step.requires().is_empty() && reopenable
}

/// Sorts the candidates by whether the run really contains their unlocker.
///
/// A step naming something this machine was never going to run is settled now,
/// with the reason its own probe gave. Holding it open would put a step on the
/// screen that nothing in the run can reach.
fn hold_open(
    candidates: Vec<Candidate>,
    pending: &[Box<dyn CoreImprovement>],
    settled: &mut Vec<StepReport>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Vec<Deferred> {
    let will_run: Vec<ImprovementId> = pending.iter().map(|step| step.id()).collect();
    let mut deferred = Vec::new();

    for Candidate { step, outcome } in candidates {
        let waiting_on: Vec<ImprovementId> = step
            .requires()
            .iter()
            .filter(|id| will_run.contains(id))
            .cloned()
            .collect();

        if waiting_on.is_empty() {
            finish(step.as_ref(), outcome, settled, on_event);
            continue;
        }

        let reason = outcome.detail().unwrap_or_default();
        on_event(RunEvent::Deferred {
            step: step.id(),
            name: step.name().to_owned(),
            reason: reason.clone(),
            waiting_on: waiting_on.clone(),
        });
        deferred.push(Deferred {
            step,
            reason,
            waiting_on,
        });
    }

    deferred
}

/// What the run does about one probe answer.
pub(super) enum Settled {
    /// Nothing left to do or nothing to be done: this is the verdict.
    Now(Outcome),
    /// Worth applying.
    Apply,
    /// Someone else owns the setting, and this run can take it back if the
    /// user says so. Held for the question the caller asks next.
    Contested { with: String, detail: String },
}

/// Turns one probe answer into what the run does about it.
pub(super) fn probe_outcome(step: &dyn CoreImprovement, cx: &CoreCx<'_>) -> Settled {
    match step.probe(cx) {
        Ok(Probe::Applicable) => Settled::Apply,
        // Outdated still means the step has work to do: a run upgrades it.
        Ok(Probe::UpdateAvailable { .. }) => Settled::Apply,
        Ok(Probe::AlreadyApplied { evidence }) => {
            Settled::Now(Outcome::AlreadyApplied { evidence })
        }
        Ok(Probe::NotApplicable { reason }) => Settled::Now(Outcome::NotApplicable { reason }),
        Ok(Probe::Conflict {
            with,
            detail,
            yours,
        }) => {
            // A conflict names the stop that would clear it only when this run
            // can take the seat back cleanly. With one, the user gets to
            // decide; without one the run stands down, because taking over a
            // scheduler it could not put back would be worse than leaving it.
            if yours.is_some() {
                Settled::Contested { with, detail }
            } else {
                Settled::Now(Outcome::Skipped {
                    reason: SkipReason::Conflict {
                        with,
                        detail,
                        yours,
                    },
                })
            }
        }
        Ok(Probe::Unknown { reason }) => Settled::Now(Outcome::Skipped {
            reason: SkipReason::CouldNotTell { detail: reason },
        }),
        Err(error) => Settled::Now(Outcome::Failed {
            error: error.describe(),
            rolled_back: RollbackStatus::NotAttempted,
        }),
    }
}

#[cfg(test)]
#[path = "probe_test.rs"]
mod probe_test;
