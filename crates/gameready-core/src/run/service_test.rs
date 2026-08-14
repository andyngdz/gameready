use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::improvement::{
    ApplyCx, Check, Dependency, Improvement, PlannedAction, PlannedPackage, Privilege, Probe,
    RollbackStatus, SkipReason, StepError, StepPlan, Verification,
};
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths};
use crate::run::RunStatus;

/// A step whose behaviour each test dials in, so the executor's contract is
/// tested rather than any one real step's logic.
pub(super) struct Fake {
    pub(super) id: &'static str,
    pub(super) probe_result: Probe,
    pub(super) applies: bool,
    pub(super) verifies: bool,
    pub(super) deps: Vec<Dependency>,
    /// Packages this step installs in its own `apply`, the way
    /// `core.pkg.tools` does, rather than declaring them as prerequisites.
    pub(super) self_installs: Vec<String>,
}

impl Fake {
    pub(super) fn applicable(id: &'static str) -> Self {
        Self {
            id,
            probe_result: Probe::Applicable,
            applies: true,
            verifies: true,
            deps: Vec::new(),
            self_installs: Vec::new(),
        }
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
        "exists so the executor can be tested without a real system change"
    }
    fn privilege(&self) -> Privilege {
        Privilege::User
    }
    fn dependencies(&self) -> &[Dependency] {
        &self.deps
    }
}

impl CoreImprovement for Fake {
    fn probe(&self, _cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        Ok(self.probe_result.clone())
    }

    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let plan = StepPlan::new(self.id(), "a fake change");
        if self.self_installs.is_empty() {
            return Ok(plan);
        }
        Ok(plan.action(PlannedAction::InstallPackages {
            packages: self
                .self_installs
                .iter()
                .map(|name| PlannedPackage {
                    name: name.clone(),
                    what: "a fake package".to_owned(),
                    why: "so the plan has something to ask about".to_owned(),
                    approx_bytes: 1_000_000,
                })
                .collect(),
            already_present: Vec::new(),
        }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
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
        if self.applies {
            return Ok(());
        }
        Err(StepError::PreconditionLost {
            step: self.id(),
            detail: "asked to fail".to_owned(),
        })
    }

    fn verify(&self, _cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let actual = if self.verifies { "yes" } else { "no" };
        Ok(Verification::new().check(Check::equals("fake", "yes", actual)))
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

pub(super) fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

fn run_with(steps: Vec<Box<dyn CoreImprovement>>, mode: Mode, runner: &MockRunner) -> RunReport {
    let dir = TempDir::new().expect("temp dir");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    execute(
        steps,
        &CoreCx::new(&facts(), runner),
        &mut journal,
        mode,
        InstallConsent::Declined,
        &[],
        &mut |_| {},
    )
    .expect("run completes")
}

#[test]
fn a_dry_run_changes_nothing() {
    let runner = MockRunner::new();
    let report = run_with(
        vec![Box::new(Fake::applicable("test.a"))],
        Mode::DryRun,
        &runner,
    );

    assert!(runner.commands().is_empty());
    assert!(runner.paths().is_empty(), "a dry run wrote a file");
    assert_eq!(report.applied(), 0);
}

#[test]
fn an_applicable_step_applies_and_verifies() {
    let runner = MockRunner::new();
    let report = run_with(
        vec![Box::new(Fake::applicable("test.a"))],
        Mode::Apply,
        &runner,
    );

    assert_eq!(report.applied(), 1);
    assert_eq!(report.failed(), 0);
    assert_eq!(report.status(), RunStatus::Clean);
}

#[test]
fn a_step_whose_verification_fails_is_rolled_back_not_reported_as_applied() {
    let runner = MockRunner::new();
    let step = Fake {
        id: "test.a",
        probe_result: Probe::Applicable,
        applies: true,
        verifies: false,
        deps: Vec::new(),
        self_installs: Vec::new(),
    };
    let report = run_with(vec![Box::new(step)], Mode::Apply, &runner);

    assert_eq!(report.applied(), 0);
    assert_eq!(report.failed(), 1);
    // The change it made was undone rather than left behind.
    assert!(
        runner.paths().is_empty(),
        "verification failed but the change survived"
    );
    match &report.steps[0].outcome {
        Outcome::Failed { rolled_back, .. } => {
            assert_eq!(*rolled_back, RollbackStatus::Succeeded);
        }
        other @ (Outcome::Applied { .. }
        | Outcome::AlreadyApplied { .. }
        | Outcome::Skipped { .. }
        | Outcome::NotApplicable { .. }) => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_failing_step_does_not_stop_the_ones_after_it() {
    let runner = MockRunner::new();
    let failing = Fake {
        id: "test.a",
        probe_result: Probe::Applicable,
        applies: false,
        verifies: true,
        deps: Vec::new(),
        self_installs: Vec::new(),
    };
    let report = run_with(
        vec![Box::new(failing), Box::new(Fake::applicable("test.b"))],
        Mode::Apply,
        &runner,
    );

    assert_eq!(report.steps.len(), 2);
    assert_eq!(report.failed(), 1);
    assert_eq!(report.applied(), 1);
}

#[test]
fn an_already_applied_step_is_not_reapplied() {
    let runner = MockRunner::new();
    let step = Fake {
        id: "test.a",
        probe_result: Probe::AlreadyApplied {
            evidence: "already 2147483642".to_owned(),
        },
        applies: true,
        verifies: true,
        deps: Vec::new(),
        self_installs: Vec::new(),
    };
    let report = run_with(vec![Box::new(step)], Mode::Apply, &runner);

    assert!(
        runner.paths().is_empty(),
        "an already-applied step still wrote"
    );
    assert!(matches!(
        report.steps[0].outcome,
        Outcome::AlreadyApplied { .. }
    ));
}

#[test]
fn an_unreadable_probe_never_becomes_permission_to_apply() {
    // A step that cannot read the current state cannot restore it.
    let runner = MockRunner::new();
    let step = Fake {
        id: "test.a",
        probe_result: Probe::Unknown {
            reason: "cannot read".to_owned(),
        },
        applies: true,
        verifies: true,
        deps: Vec::new(),
        self_installs: Vec::new(),
    };
    let report = run_with(vec![Box::new(step)], Mode::Apply, &runner);

    assert!(runner.paths().is_empty());
    // A skip rather than a not-applicable: this machine may well take the
    // step, and saying otherwise would settle a question nothing answered.
    assert!(matches!(
        report.steps[0].outcome,
        Outcome::Skipped {
            reason: SkipReason::CouldNotTell { .. }
        }
    ));
}

#[test]
fn a_run_with_no_applicable_steps_reports_that_distinctly() {
    let runner = MockRunner::new();
    let report = run_with(vec![], Mode::Apply, &runner);
    assert_eq!(report.status(), RunStatus::NothingApplicable);
    assert_eq!(report.status().code(), 3);
}

#[test]
fn an_agreed_takeover_runs_the_step_and_reports_it_applied() {
    let runner = MockRunner::new();
    let dir = TempDir::new().expect("temp dir");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    let step = Fake {
        probe_result: Probe::Conflict {
            with: "cosmos".to_owned(),
            detail: "cosmos is already scheduling this machine".to_owned(),
            yours: Some("scxctl stop".to_owned()),
        },
        ..Fake::applicable("test.owned")
    };

    let report = execute(
        vec![Box::new(step)],
        &CoreCx::new(&facts(), &runner),
        &mut journal,
        Mode::Apply,
        InstallConsent::Declined,
        &[ImprovementId::from_static("test.owned")],
        &mut |_| {},
    )
    .expect("run completes");

    assert!(matches!(report.steps[0].outcome, Outcome::Applied { .. }));
    assert!(runner.paths().contains(&"/tmp/fake".into()));
}

#[test]
fn a_declined_takeover_stands_down_with_the_conflict_words() {
    let runner = MockRunner::new();
    let dir = TempDir::new().expect("temp dir");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    let step = Fake {
        probe_result: Probe::Conflict {
            with: "cosmos".to_owned(),
            detail: "cosmos is already scheduling this machine".to_owned(),
            yours: Some("scxctl stop".to_owned()),
        },
        ..Fake::applicable("test.owned")
    };

    let report = execute(
        vec![Box::new(step)],
        &CoreCx::new(&facts(), &runner),
        &mut journal,
        Mode::Apply,
        InstallConsent::Declined,
        &[],
        &mut |_| {},
    )
    .expect("run completes");

    // Declined takeovers change nothing, and the summary keeps the words the
    // probe found rather than inventing a new reason.
    assert!(runner.paths().is_empty());
    assert!(matches!(
        &report.steps[0].outcome,
        Outcome::Skipped {
            reason: SkipReason::Conflict { with, .. }
        } if with == "cosmos"
    ));
}

#[test]
fn events_arrive_in_phase_order() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    let mut seen = Vec::new();

    execute(
        vec![Box::new(Fake::applicable("test.a"))],
        &CoreCx::new(&facts(), &runner),
        &mut journal,
        Mode::Apply,
        InstallConsent::Declined,
        &[],
        &mut |event| seen.push(event),
    )
    .expect("run completes");

    // Everything is probed and resolved before anything applies, so the plan
    // the user sees is complete before the first change.
    assert!(matches!(seen[0], RunEvent::Probing { .. }));
    assert!(matches!(seen[1], RunEvent::Planned { .. }));
    assert!(matches!(seen[2], RunEvent::DependenciesResolved { .. }));
    assert!(matches!(seen[3], RunEvent::Applying { .. }));
    assert!(matches!(seen[4], RunEvent::Finished { .. }));
}
