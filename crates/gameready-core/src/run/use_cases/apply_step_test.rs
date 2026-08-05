use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::improvement::{
    Check, CoreImprovement, Improvement, ImprovementId, Privilege, Probe, StepPlan, Verification,
};
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths};

struct FakeStep {
    applies: bool,
    verifies: bool,
}

impl Improvement for FakeStep {
    fn id(&self) -> ImprovementId {
        ImprovementId::from_static("test.apply-step")
    }
    fn name(&self) -> &str {
        "fake"
    }
    fn rationale(&self) -> &str {
        "test"
    }
    fn privilege(&self) -> Privilege {
        Privilege::User
    }
}

impl CoreImprovement for FakeStep {
    fn probe(&self, _cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        Ok(Probe::Applicable)
    }
    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "fake"))
    }
    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        cx.mutate(
            Change::DirCreated {
                path: "/tmp/fake-apply".into(),
            },
            |runner| {
                runner
                    .write_file("/tmp/fake-apply".as_ref(), "x", Privilege::User)
                    .map_err(|source| StepError::Write {
                        path: "/tmp/fake-apply".into(),
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
    fn rollback(
        &self,
        _undo: &[Change],
        cx: &mut ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        cx.reader()
            .remove_file("/tmp/fake-apply".as_ref(), Privilege::User)
            .map_err(|source| StepError::Write {
                path: "/tmp/fake-apply".into(),
                source: std::io::Error::other(source.to_string()),
            })
    }
}

fn run(step: &FakeStep) -> Outcome {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal");
    apply_and_verify(step, &cx, &runner, &mut journal)
}

#[test]
fn successful_apply_reports_applied() {
    let outcome = run(&FakeStep {
        applies: true,
        verifies: true,
    });
    assert!(matches!(outcome, Outcome::Applied { .. }));
}

#[test]
fn failed_apply_rolls_back() {
    let outcome = run(&FakeStep {
        applies: false,
        verifies: true,
    });
    match outcome {
        Outcome::Failed { rolled_back, .. } => {
            assert_eq!(rolled_back, RollbackStatus::Succeeded);
        }
        other @ (Outcome::Applied { .. }
        | Outcome::AlreadyApplied { .. }
        | Outcome::Skipped { .. }
        | Outcome::NotApplicable { .. }) => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn failed_verification_rolls_back() {
    let outcome = run(&FakeStep {
        applies: true,
        verifies: false,
    });
    match outcome {
        Outcome::Failed { rolled_back, .. } => {
            assert_eq!(rolled_back, RollbackStatus::Succeeded);
        }
        other @ (Outcome::Applied { .. }
        | Outcome::AlreadyApplied { .. }
        | Outcome::Skipped { .. }
        | Outcome::NotApplicable { .. }) => panic!("expected Failed, got {other:?}"),
    }
}
