use super::*;
use crate::facts::{Family, SystemFacts};
use crate::improvement::{ApplyCx, Improvement, Privilege, StepError, StepPlan, Verification};
use crate::infra::exec::MockRunner;
use crate::journal::Change;

/// A step whose probe answer and `requires()` list a test dictates.
struct Fake {
    id: &'static str,
    probe_result: Probe,
    requires: Vec<ImprovementId>,
}

impl Fake {
    fn applicable(id: &'static str) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result: Probe::Applicable,
            requires: Vec::new(),
        })
    }

    fn ruled_out(
        id: &'static str,
        reason: &str,
        requires: &[&'static str],
    ) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result: Probe::NotApplicable {
                reason: reason.to_owned(),
            },
            requires: requires
                .iter()
                .map(|id| ImprovementId::from_static(id))
                .collect(),
        })
    }

    fn already_applied(id: &'static str, requires: &[&'static str]) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result: Probe::AlreadyApplied {
                evidence: "nothing to do".to_owned(),
            },
            requires: requires
                .iter()
                .map(|id| ImprovementId::from_static(id))
                .collect(),
        })
    }

    fn update_available(id: &'static str, requires: &[&'static str]) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result: Probe::UpdateAvailable {
                installed: "GE-Proton11-3".to_owned(),
                latest: "GE-Proton11-5".to_owned(),
            },
            requires: requires
                .iter()
                .map(|id| ImprovementId::from_static(id))
                .collect(),
        })
    }

    /// A step somebody else owns, with or without a path back for a takeover.
    fn conflicted(id: &'static str, takeover_possible: bool) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result: Probe::Conflict {
                with: "tuned.service".to_owned(),
                detail: "tuned.service sets the governor on its own schedule".to_owned(),
                yours: takeover_possible
                    .then(|| "systemctl disable --now tuned.service".to_owned()),
            },
            requires: Vec::new(),
        })
    }
}

impl Improvement for Fake {
    fn id(&self) -> ImprovementId {
        ImprovementId::from_static(self.id)
    }
    fn name(&self) -> &str {
        "fake step"
    }
    fn rationale(&self) -> &str {
        "exists so probing can be tested without touching a system"
    }
    fn privilege(&self) -> Privilege {
        Privilege::User
    }
    fn requires(&self) -> &[ImprovementId] {
        &self.requires
    }
}

impl CoreImprovement for Fake {
    fn probe(&self, _cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        Ok(self.probe_result.clone())
    }
    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "a fake change"))
    }
    fn apply(&self, _cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        Ok(())
    }
    fn verify(&self, _cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        Ok(Verification::new())
    }
    fn rollback(
        &self,
        _undo: &[Change],
        _cx: &mut ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        Ok(())
    }
}

fn sort(steps: Vec<Box<dyn CoreImprovement>>) -> (Probed, Vec<RunEvent>) {
    let runner = MockRunner::new();
    let system = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&system, &runner);

    let mut events = Vec::new();
    let probed = probe_all(steps, &cx, &mut |event| events.push(event));
    (probed, events)
}

#[test]
fn a_ruled_out_step_whose_unlocker_is_running_is_held_open_rather_than_settled() {
    let (probed, _) = sort(vec![
        Fake::applicable("test.unlocker"),
        Fake::ruled_out("test.waiter", "not yet", &["test.unlocker"]),
    ]);

    assert_eq!(probed.pending.len(), 1);
    assert!(probed.settled.is_empty());
    assert_eq!(probed.deferred.len(), 1);
    assert_eq!(probed.deferred[0].step.id().as_str(), "test.waiter");
    assert_eq!(probed.deferred[0].reason, "not yet");
    assert_eq!(
        probed.deferred[0].waiting_on,
        vec![ImprovementId::from_static("test.unlocker")]
    );
}

#[test]
fn a_step_nothing_in_this_run_could_unlock_is_settled_at_probe_time_as_before() {
    // The unlocker it names probed as already applied, so it never runs, so a
    // second look would read exactly the same system.
    let (probed, _) = sort(vec![
        Fake::already_applied("test.unlocker", &[]),
        Fake::ruled_out("test.waiter", "not here", &["test.unlocker"]),
    ]);

    assert!(probed.deferred.is_empty());
    assert_eq!(probed.settled.len(), 2);
}

#[test]
fn a_step_that_names_nobody_is_settled_even_when_something_else_is_running() {
    let (probed, _) = sort(vec![
        Fake::applicable("test.unlocker"),
        Fake::ruled_out("test.loner", "not for this machine", &[]),
    ]);

    assert!(probed.deferred.is_empty());
    assert_eq!(probed.settled.len(), 1);
}

#[test]
fn an_outdated_install_is_still_work_a_run_would_do() {
    let (probed, _) = sort(vec![Fake::update_available("test.old", &[])]);

    assert_eq!(probed.pending.len(), 1);
    assert!(probed.settled.is_empty());
}

#[test]
fn a_step_that_is_already_applied_is_never_held_open() {
    // Holding it would put a step back on the list that has nothing left to do.
    let (probed, _) = sort(vec![
        Fake::applicable("test.unlocker"),
        Fake::already_applied("test.done", &["test.unlocker"]),
    ]);

    assert!(probed.deferred.is_empty());
    assert_eq!(probed.settled.len(), 1);
}

#[test]
fn a_conflict_the_run_can_clear_is_held_for_the_takeover_question() {
    let (probed, _) = sort(vec![Fake::conflicted("test.owned", true)]);

    assert!(probed.settled.is_empty(), "{:?}", probed.settled);
    assert_eq!(probed.contested.len(), 1);
    assert_eq!(probed.contested[0].step.id().as_str(), "test.owned");
    assert_eq!(probed.contested[0].with, "tuned.service");
}

#[test]
fn a_conflict_without_a_path_back_is_settled_as_the_skip_it_was() {
    // The run could stop the owner, so it has no right to take the seat; the
    // step stands down the way it always did, with the same words.
    let (probed, _) = sort(vec![Fake::conflicted("test.owned", false)]);

    assert!(probed.contested.is_empty());
    assert_eq!(probed.settled.len(), 1);
    assert!(matches!(
        &probed.settled[0].outcome,
        Outcome::Skipped {
            reason: SkipReason::Conflict { with, .. }
        } if with == "tuned.service"
    ));
}

#[test]
fn a_held_open_step_reports_itself_rather_than_disappearing_from_the_screen() {
    let (_, events) = sort(vec![
        Fake::applicable("test.unlocker"),
        Fake::ruled_out("test.waiter", "not yet", &["test.unlocker"]),
    ]);

    let deferred = events
        .iter()
        .find(|event| matches!(event, RunEvent::Deferred { .. }))
        .expect("a held-open step announces itself");
    assert!(matches!(
        deferred,
        RunEvent::Deferred { reason, .. } if reason == "not yet"
    ));
}

#[test]
fn a_held_open_step_counts_as_applicable_rather_than_as_skipped() {
    let (_, events) = sort(vec![
        Fake::applicable("test.unlocker"),
        Fake::ruled_out("test.waiter", "not yet", &["test.unlocker"]),
    ]);

    let planned = events
        .iter()
        .find(|event| matches!(event, RunEvent::Planned { .. }))
        .expect("planning ends with a count");
    assert!(matches!(
        planned,
        RunEvent::Planned {
            applicable: 2,
            skipped: 0
        }
    ));
}

#[test]
fn no_step_reports_finished_twice() {
    // A held-open step is announced as deferred, never also as finished, or the
    // summary would list it under a verdict the run is about to overturn.
    let (_, events) = sort(vec![
        Fake::applicable("test.unlocker"),
        Fake::ruled_out("test.waiter", "not yet", &["test.unlocker"]),
    ]);

    let finished = events
        .iter()
        .filter(|event| matches!(event, RunEvent::Finished { .. }))
        .count();
    assert_eq!(finished, 0);
}
