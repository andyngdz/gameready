use super::*;
use crate::facts::{Family, SystemFacts};
use crate::improvement::{
    ApplyCx, Dependency, DependencyKind, Improvement, ImprovementId, PackageSpec, Privilege, Probe,
    StepError, StepPlan, Verification,
};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::Apt;
use crate::journal::Change;
use crate::run::domain::DependencyStatus;

/// A step that always applies and declares whatever dependencies a test hands
/// it, so planning is exercised without a real system change.
struct Fake {
    id: &'static str,
    probe_result: Probe,
    deps: Vec<Dependency>,
}

impl Fake {
    fn applicable(id: &'static str, deps: Vec<Dependency>) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result: Probe::Applicable,
            deps,
        })
    }

    fn probing(id: &'static str, probe_result: Probe) -> Box<dyn CoreImprovement> {
        Box::new(Self {
            id,
            probe_result,
            deps: Vec::new(),
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
        "exists so planning can be tested without touching a system"
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

fn package_dep(name: &'static str) -> Dependency {
    Dependency::new(
        DependencyKind::Package {
            spec: PackageSpec::uniform(name, 5_000_000),
        },
        "a test package",
        "so the plan has something to resolve",
    )
}

#[test]
fn planning_reports_what_is_pending_and_what_is_already_settled() {
    let runner = MockRunner::new();
    let system = SystemFacts::fixture(Family::Debian);
    let packages = Apt;
    let cx = CoreCx::new(&system, &runner).with_packages(&packages);

    let plan = plan_run(
        vec![
            Fake::applicable("test.a", Vec::new()),
            Fake::probing(
                "test.b",
                Probe::AlreadyApplied {
                    evidence: "already set".to_owned(),
                },
            ),
        ],
        &cx,
        &mut |_| {},
    );

    assert_eq!(plan.pending.len(), 1);
    assert_eq!(plan.settled.len(), 1);
    assert_eq!(plan.settled[0].step.as_str(), "test.b");
}

#[test]
fn a_step_whose_package_this_distro_lacks_is_not_applicable_before_anything_is_asked() {
    // Nothing the user could answer makes the package appear, so this is settled
    // during planning rather than being put to them as a choice.
    let runner = MockRunner::new();
    let system = SystemFacts::fixture(Family::Debian);
    let packages = Apt;
    let cx = CoreCx::new(&system, &runner).with_packages(&packages);

    let plan = plan_run(
        vec![Fake::applicable("test.a", vec![package_dep("scx-scheds")])],
        &cx,
        &mut |_| {},
    );

    assert!(plan.pending.is_empty());
    assert_eq!(plan.settled.len(), 1);
    assert!(matches!(
        plan.settled[0].outcome,
        Outcome::NotApplicable { .. }
    ));
    assert_eq!(
        plan.preflight.dependencies[0].status,
        DependencyStatus::Unavailable
    );
}

#[test]
fn planning_without_package_tooling_reports_nothing_to_install() {
    let runner = MockRunner::new();
    let system = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&system, &runner);

    let plan = plan_run(
        vec![Fake::applicable("test.a", vec![package_dep("mangohud")])],
        &cx,
        &mut |_| {},
    );

    assert!(!plan.installs_anything());
    assert_eq!(plan.pending.len(), 1);
}

#[test]
fn planning_is_the_same_whatever_mode_the_run_is_in() {
    let runner = MockRunner::new();
    let system = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&system, &runner);

    let plan = plan_run(
        vec![Fake::applicable("test.a", Vec::new())],
        &cx,
        &mut |_| {},
    );

    // Planning does not consult the mode, so a dry run reaches the same list a
    // real run would and can say what it would have installed.
    assert_eq!(plan.pending.len(), 1);
    assert!(plan.settled.is_empty());
}
