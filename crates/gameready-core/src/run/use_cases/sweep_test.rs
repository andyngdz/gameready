use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::improvement::OutcomeKind;
use crate::improvement::{
    ApplyCx, Check, Improvement, Privilege, Probe, StepError, StepPlan, Verification,
};
use crate::infra::exec::MockRunner;
use crate::journal::{Change, RunId, StatePaths};

/// One step's probe answer, shared so another step's apply can change it.
///
/// This is the whole point of the feature in miniature: what the machine says
/// about a step is not fixed for the length of a run.
type Answer = Arc<Mutex<Probe>>;

fn answer(probe: Probe) -> Answer {
    Arc::new(Mutex::new(probe))
}

/// A step whose probe reads a shared answer and whose apply can rewrite
/// somebody else's.
struct Fake {
    id: &'static str,
    requires: Vec<ImprovementId>,
    answer: Answer,
    probes: Arc<AtomicUsize>,
    applied: Arc<AtomicUsize>,
    unlocks: Vec<(Answer, Probe)>,
    succeeds: bool,
}

impl Fake {
    fn new(id: &'static str, probe: Probe) -> Self {
        Self {
            id,
            requires: Vec::new(),
            answer: answer(probe),
            probes: Arc::new(AtomicUsize::new(0)),
            applied: Arc::new(AtomicUsize::new(0)),
            unlocks: Vec::new(),
            succeeds: true,
        }
    }

    fn waiting_for(mut self, unlocker: &'static str) -> Self {
        self.requires.push(ImprovementId::from_static(unlocker));
        self
    }

    fn unlocking(mut self, target: &Answer, into: Probe) -> Self {
        self.unlocks.push((Arc::clone(target), into));
        self
    }

    fn failing(mut self) -> Self {
        self.succeeds = false;
        self
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
        "exists so the sweep can be tested without a real system change"
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
        self.probes.fetch_add(1, Ordering::SeqCst);
        let held = self
            .answer
            .lock()
            .map_err(|_| StepError::PreconditionLost {
                step: self.id(),
                detail: "answer poisoned".to_owned(),
            })?;
        Ok(held.clone())
    }

    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "a fake change"))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        self.applied.fetch_add(1, Ordering::SeqCst);
        cx.mutate(
            Change::DirCreated {
                path: "/tmp/fake".into(),
                privilege: Privilege::User,
            },
            |runner| {
                runner
                    .write_file("/tmp/fake".as_ref(), "x", Privilege::User)
                    .map_err(StepError::Exec)
            },
        )?;

        if !self.succeeds {
            return Err(StepError::PreconditionLost {
                step: self.id(),
                detail: "asked to fail".to_owned(),
            });
        }

        for (target, into) in &self.unlocks {
            if let Ok(mut held) = target.lock() {
                *held = into.clone();
            }
        }
        Ok(())
    }

    fn verify(&self, _cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        Ok(Verification::new().check(Check::equals("fake", "yes", "yes")))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        for _ in undo {
            cx.reader()
                .remove_file("/tmp/fake".as_ref(), Privilege::User)
                .map_err(StepError::Exec)?;
        }
        Ok(())
    }
}

fn held(step: Fake, reason: &str) -> Deferred {
    let waiting_on = step.requires.clone();
    Deferred {
        step: Box::new(step),
        reason: reason.to_owned(),
        waiting_on,
    }
}

struct Swept {
    settled: Vec<StepReport>,
    events: Vec<RunEvent>,
}

impl Swept {
    fn outcome(&self, id: &str) -> &Outcome {
        &self
            .settled
            .iter()
            .find(|report| report.step.as_str() == id)
            .unwrap_or_else(|| panic!("{id} is missing from the report"))
            .outcome
    }

    fn rechecks(&self) -> usize {
        self.events
            .iter()
            .filter(|event| matches!(event, RunEvent::Reprobing { .. }))
            .count()
    }
}

fn sweep(pending: Vec<Box<dyn CoreImprovement>>, deferred: Vec<Deferred>) -> Swept {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let system = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&system, &runner);
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let mut settled = Vec::new();
    let mut events = Vec::new();
    apply_all(
        pending,
        deferred,
        &cx,
        &mut journal,
        &mut settled,
        &mut |event| events.push(event),
    )
    .expect("the sweep completes");

    Swept { settled, events }
}

#[test]
fn a_step_the_probe_ruled_out_gets_another_look_after_the_step_it_named_applies() {
    let waiter_answer = answer(Probe::NotApplicable {
        reason: "the package is not in your repositories".to_owned(),
    });
    let unlocker =
        Fake::new("test.ppa", Probe::Applicable).unlocking(&waiter_answer, Probe::Applicable);
    let waiter = Fake::new("test.sched", Probe::Applicable).waiting_for("test.ppa");
    let waiter_applied = Arc::clone(&waiter.applied);
    let waiter = Fake {
        answer: Arc::clone(&waiter_answer),
        ..waiter
    };

    let swept = sweep(
        vec![Box::new(unlocker)],
        vec![held(waiter, "the package is not in your repositories")],
    );

    assert!(matches!(
        swept.outcome("test.sched"),
        Outcome::Applied { .. }
    ));
    assert_eq!(waiter_applied.load(Ordering::SeqCst), 1);
}

#[test]
fn a_step_still_ruled_out_after_the_unlock_reports_what_the_second_probe_found() {
    let waiter = Fake::new(
        "test.sched",
        Probe::NotApplicable {
            reason: "this kernel is too old for it".to_owned(),
        },
    )
    .waiting_for("test.ppa");

    let swept = sweep(
        vec![Box::new(Fake::new("test.ppa", Probe::Applicable))],
        vec![held(waiter, "the package is not in your repositories")],
    );

    // The second probe's reason, not the first one's, or the summary would
    // explain the run with a fact that stopped being true halfway through it.
    assert!(matches!(
        swept.outcome("test.sched"),
        Outcome::NotApplicable { reason } if reason == "this kernel is too old for it"
    ));
}

#[test]
fn a_step_whose_unlock_failed_is_skipped_rather_than_probed_again() {
    // Probing a system a failed step may have half-changed reads a state
    // nobody meant to create.
    let waiter = Fake::new("test.sched", Probe::Applicable).waiting_for("test.ppa");
    let waiter_probes = Arc::clone(&waiter.probes);

    let swept = sweep(
        vec![Box::new(Fake::new("test.ppa", Probe::Applicable).failing())],
        vec![held(waiter, "the package is not in your repositories")],
    );

    assert_eq!(waiter_probes.load(Ordering::SeqCst), 0);
    assert!(matches!(
        swept.outcome("test.sched"),
        Outcome::Skipped {
            reason: SkipReason::DependencyFailed { .. }
        }
    ));
}

#[test]
fn a_held_open_step_is_probed_once_more_and_not_once_per_step_that_runs() {
    let waiter = Fake::new("test.sched", Probe::Applicable).waiting_for("test.ppa");
    let waiter_probes = Arc::clone(&waiter.probes);

    sweep(
        vec![
            Box::new(Fake::new("test.ppa", Probe::Applicable)),
            Box::new(Fake::new("test.other", Probe::Applicable)),
            Box::new(Fake::new("test.third", Probe::Applicable)),
        ],
        vec![held(waiter, "not yet")],
    );

    assert_eq!(waiter_probes.load(Ordering::SeqCst), 1);
}

#[test]
fn a_step_made_unnecessary_by_an_earlier_one_does_not_apply() {
    // The other direction: gamemode lands, so the step that would have pinned
    // the governor finds somebody else already doing it.
    let governor_answer = answer(Probe::Applicable);
    let tools = Fake::new("test.tools", Probe::Applicable).unlocking(
        &governor_answer,
        Probe::AlreadyApplied {
            evidence: "gamemode raises the governor while a game runs".to_owned(),
        },
    );
    let governor = Fake {
        answer: Arc::clone(&governor_answer),
        ..Fake::new("test.governor", Probe::Applicable).waiting_for("test.tools")
    };
    let governor_applied = Arc::clone(&governor.applied);

    let swept = sweep(vec![Box::new(tools), Box::new(governor)], Vec::new());

    assert_eq!(governor_applied.load(Ordering::SeqCst), 0);
    assert_eq!(
        swept.outcome("test.governor").kind(),
        OutcomeKind::AlreadySet
    );
}

#[test]
fn a_step_that_names_nobody_is_never_probed_twice() {
    let step = Fake::new("test.plain", Probe::Applicable);
    let probes = Arc::clone(&step.probes);

    sweep(vec![Box::new(step)], Vec::new());

    // probe_all already ran outside this sweep, so a second probe here would
    // be the third look at a step nothing in the run can affect.
    assert_eq!(probes.load(Ordering::SeqCst), 0);
}

#[test]
fn the_recheck_names_the_step_that_made_it_worth_looking_again() {
    let waiter = Fake::new("test.sched", Probe::Applicable).waiting_for("test.ppa");

    let swept = sweep(
        vec![Box::new(Fake::new("test.ppa", Probe::Applicable))],
        vec![held(waiter, "not yet")],
    );

    assert_eq!(swept.rechecks(), 1);
    let recheck = swept
        .events
        .iter()
        .find_map(|event| match event {
            RunEvent::Reprobing { step, after } => Some((step.clone(), after.clone())),
            RunEvent::Probing { .. }
            | RunEvent::Deferred { .. }
            | RunEvent::Planned { .. }
            | RunEvent::DependenciesResolved { .. }
            | RunEvent::InstallingDependencies { .. }
            | RunEvent::DependenciesInstalled { .. }
            | RunEvent::Applying { .. }
            | RunEvent::StepProgress { .. }
            | RunEvent::StepBytes { .. }
            | RunEvent::Finished { .. } => None,
        })
        .expect("one re-check");
    assert_eq!(recheck.0.as_str(), "test.sched");
    assert_eq!(recheck.1.as_str(), "test.ppa");
}

#[test]
fn a_held_open_step_whose_unlocker_never_ran_keeps_the_reason_it_was_given() {
    // The user declined the install, so the step it waited on was dropped
    // before the sweep started and nothing will ever release it.
    let waiter = Fake::new("test.sched", Probe::Applicable).waiting_for("test.ppa");

    let swept = sweep(Vec::new(), vec![held(waiter, "not in your repositories")]);

    assert!(matches!(
        swept.outcome("test.sched"),
        Outcome::NotApplicable { reason } if reason == "not in your repositories"
    ));
}
