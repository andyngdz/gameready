//! Applying an agreed plan, looking again as the machine changes underneath it.
//!
//! One pass, not one pass per step. A step that installs something can make an
//! earlier verdict wrong in both directions: a tuning ruled out for want of a
//! package becomes possible, and a tuning worth doing becomes unnecessary
//! because the thing just installed now does it. Both are settled here, while
//! the run is still in front of the user, rather than left for the next one.

use std::collections::VecDeque;

use crate::improvement::{CoreCx, CoreImprovement, Doing, ImprovementId, Outcome, SkipReason};
use crate::journal::{Journal, JournalEvent};
use crate::run::domain::{Deferred, RunEvent, StepReport};
use crate::run::errors::RunError;
use crate::run::use_cases::apply_step::apply_and_verify;
use crate::run::use_cases::probe::{finish, probe_outcome, Settled};

/// Applies every step that can still run, releasing held-open ones as it goes.
pub(crate) fn apply_all(
    pending: Vec<Box<dyn CoreImprovement>>,
    deferred: Vec<Deferred>,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    settled: &mut Vec<StepReport>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<(), RunError> {
    let mut queue: VecDeque<Queued> = pending.into_iter().map(Queued::planned).collect();
    let mut held = deferred;
    let mut ran = Ran::default();

    while let Some(Queued { step, look }) = queue.pop_front() {
        let id = step.id();
        match second_look(step.as_ref(), cx, &ran, &look, on_event) {
            Some(outcome) => finish(step.as_ref(), outcome, settled, on_event),
            None => {
                let outcome = apply_one(step.as_ref(), cx, journal, on_event)?;
                ran.record(id.clone(), &outcome);
                finish(step.as_ref(), outcome, settled, on_event);
            }
        }
        release(&mut held, &ran, &id, cx, settled, &mut queue, on_event);
    }

    // Whatever is left waited on a step that never ran, usually because the
    // user declined the install that would have carried it.
    for Deferred { step, reason, .. } in held {
        finish(
            step.as_ref(),
            Outcome::NotApplicable { reason },
            settled,
            on_event,
        );
    }
    Ok(())
}

/// A step waiting its turn, and whether the sweep has already looked at it a
/// second time.
///
/// A released step was probed on the way out of the held list, so probing it
/// again on the way in would be the third look at the same machine. That is
/// what keeps the cost one extra probe per step, not one per step that runs.
struct Queued {
    step: Box<dyn CoreImprovement>,
    look: Look,
}

/// How many times this run has asked the machine about one step.
enum Look {
    /// Once, during planning. It is still owed a second look.
    Planned,
    /// Twice: this step came back off the held list, which is where its
    /// second look happened.
    Released,
}

impl Queued {
    fn planned(step: Box<dyn CoreImprovement>) -> Self {
        Self {
            step,
            look: Look::Planned,
        }
    }

    fn released(step: Box<dyn CoreImprovement>) -> Self {
        Self {
            step,
            look: Look::Released,
        }
    }
}

/// What the sweep has learned about the steps that already ran.
#[derive(Default)]
struct Ran {
    succeeded: Vec<ImprovementId>,
    failed: Vec<ImprovementId>,
}

impl Ran {
    fn record(&mut self, step: ImprovementId, outcome: &Outcome) {
        if outcome.is_failure() {
            self.failed.push(step);
        } else {
            self.succeeded.push(step);
        }
    }

    fn done(&self, step: &ImprovementId) -> bool {
        self.succeeded.contains(step) || self.failed.contains(step)
    }
}

/// Whether something that ran since has made this step unnecessary.
///
/// Asked only of a step that named an unlocker, only once one of those has
/// succeeded, and never of a step the sweep has already looked at twice. It
/// can only cancel work, never add any: a step that still probes applicable
/// goes on to apply exactly as it would have, so this needs no consent
/// argument of its own.
fn second_look(
    step: &dyn CoreImprovement,
    cx: &CoreCx<'_>,
    ran: &Ran,
    look: &Look,
    on_event: &mut dyn FnMut(RunEvent),
) -> Option<Outcome> {
    match look {
        Look::Released => return None,
        Look::Planned => {}
    }

    let after = step
        .requires()
        .iter()
        .find(|id| ran.succeeded.contains(id))?
        .clone();

    on_event(RunEvent::Reprobing {
        step: step.id(),
        after,
    });

    match probe_outcome(step, cx) {
        Settled::Apply => None,
        Settled::Now(outcome) => Some(outcome),
    }
}

/// Puts back every held-open step whose unlockers have all finished.
fn release(
    held: &mut Vec<Deferred>,
    ran: &Ran,
    just_finished: &ImprovementId,
    cx: &CoreCx<'_>,
    settled: &mut Vec<StepReport>,
    queue: &mut VecDeque<Queued>,
    on_event: &mut dyn FnMut(RunEvent),
) {
    for entry in drain_ready(held, ran) {
        let Deferred {
            step, waiting_on, ..
        } = entry;

        // Probing a system that a failed step may have half-changed reads a
        // state nobody meant to create, so this one is not probed at all.
        if let Some(broken) = waiting_on.iter().find(|id| ran.failed.contains(id)) {
            let outcome = Outcome::Skipped {
                reason: SkipReason::DependencyFailed { on: broken.clone() },
            };
            finish(step.as_ref(), outcome, settled, on_event);
            continue;
        }

        on_event(RunEvent::Reprobing {
            step: step.id(),
            after: just_finished.clone(),
        });

        match probe_outcome(step.as_ref(), cx) {
            Settled::Apply => queue.push_back(Queued::released(step)),
            Settled::Now(outcome) => finish(step.as_ref(), outcome, settled, on_event),
        }
    }
}

/// Takes out every held-open step with nothing left to wait for.
fn drain_ready(held: &mut Vec<Deferred>, ran: &Ran) -> Vec<Deferred> {
    let mut ready = Vec::new();
    let mut waiting = Vec::new();

    for entry in held.drain(..) {
        if entry.waiting_on.iter().all(|id| ran.done(id)) {
            ready.push(entry);
        } else {
            waiting.push(entry);
        }
    }

    *held = waiting;
    ready
}

/// Announces one step, journals its boundaries, and applies it.
fn apply_one(
    step: &dyn CoreImprovement,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<Outcome, RunError> {
    on_event(RunEvent::Applying {
        step: step.id(),
        name: step.name().to_owned(),
    });

    journal.append(JournalEvent::StepBegin { step: step.id() })?;

    let step_id = step.id();
    let progress: Box<dyn FnMut(Doing) + '_> = Box::new(|doing: Doing| {
        let step = step_id.clone();
        on_event(match doing {
            Doing::Phase(message) => RunEvent::StepProgress { step, message },
            Doing::Bytes { done, total } => RunEvent::StepBytes { step, done, total },
        });
    });
    let outcome = apply_and_verify(step, cx, cx.runner, journal, Some(progress));

    journal.append(JournalEvent::StepEnd {
        step: step.id(),
        outcome: outcome.label().to_owned(),
    })?;

    Ok(outcome)
}

#[cfg(test)]
#[path = "sweep_test.rs"]
mod sweep_test;
