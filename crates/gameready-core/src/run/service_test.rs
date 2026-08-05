use tempfile::TempDir;

use super::*;
use crate::improvement::{Check, Improvement, ImprovementId, Privilege, StepPlan, Verification};
use crate::infra::exec::MockRunner;
use crate::journal::{Change, RunId, StatePaths};
use crate::run::RunStatus;

/// A step whose behaviour each test dials in, so the executor's contract is
/// tested rather than any one real step's logic.
struct Fake {
    id: &'static str,
    probe_result: Probe,
    applies: bool,
    verifies: bool,
}

impl Fake {
    fn applicable(id: &'static str) -> Self {
        Self {
            id,
            probe_result: Probe::Applicable,
            applies: true,
            verifies: true,
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
}

impl CoreImprovement for Fake {
    fn probe(&self, _cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        Ok(self.probe_result.clone())
    }

    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "a fake change"))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        cx.mutate(
            Change::DirCreated {
                path: "/tmp/fake".into(),
            },
            |runner| {
                runner
                    .write_file("/tmp/fake".as_ref(), "x", Privilege::User)
                    .map_err(|source| StepError::Write {
                        path: "/tmp/fake".into(),
                        source: std::io::Error::other(source.to_string()),
                    })
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
                .map_err(|source| StepError::Write {
                    path: "/tmp/fake".into(),
                    source: std::io::Error::other(source.to_string()),
                })?;
        }
        Ok(())
    }
}

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

fn run_with(steps: Vec<Box<dyn CoreImprovement>>, mode: Mode, runner: &MockRunner) -> RunReport {
    let dir = TempDir::new().expect("temp dir");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    execute(steps, &facts(), runner, &mut journal, mode, &mut |_| {}).expect("run completes")
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
    };
    let report = run_with(vec![Box::new(step)], Mode::Apply, &runner);

    assert!(runner.paths().is_empty());
    assert!(matches!(
        report.steps[0].outcome,
        Outcome::NotApplicable { .. }
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
fn events_arrive_in_phase_order() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    let mut seen = Vec::new();

    execute(
        vec![Box::new(Fake::applicable("test.a"))],
        &facts(),
        &runner,
        &mut journal,
        Mode::Apply,
        &mut |event| seen.push(event),
    )
    .expect("run completes");

    // Everything is probed before anything applies, so the plan the user sees
    // is complete before the first change.
    assert!(matches!(seen[0], RunEvent::Probing { .. }));
    assert!(matches!(seen[1], RunEvent::Planned { .. }));
    assert!(matches!(seen[2], RunEvent::Applying { .. }));
    assert!(matches!(seen[3], RunEvent::Finished { .. }));
}
