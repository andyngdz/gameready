use super::*;
use crate::facts::{Family, PackageManagerKind, SystemFacts};
use crate::improvement::{
    CoreCx, CoreImprovement, Dependency, DependencyKind, Improvement, ImprovementId, PackageSpec,
    Privilege, Probe, StepError, StepPlan, Verification,
};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::Apt;
use crate::run::domain::DependencyStatus;

struct StepWithDeps {
    id: &'static str,
    deps: Vec<Dependency>,
}

impl StepWithDeps {
    fn new(id: &'static str, deps: Vec<Dependency>) -> Self {
        Self { id, deps }
    }
}

impl Improvement for StepWithDeps {
    fn id(&self) -> ImprovementId {
        ImprovementId::from_static(self.id)
    }
    fn name(&self) -> &str {
        "test step"
    }
    fn rationale(&self) -> &str {
        "test"
    }
    fn privilege(&self) -> Privilege {
        Privilege::Root
    }
    fn dependencies(&self) -> &[Dependency] {
        &self.deps
    }
}

impl CoreImprovement for StepWithDeps {
    fn probe(&self, _cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        Ok(Probe::Applicable)
    }
    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(StepPlan::new(self.id(), "test"))
    }
    fn apply(
        &self,
        _cx: &mut crate::improvement::ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        Ok(())
    }
    fn verify(&self, _cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        Ok(Verification::new())
    }
    fn rollback(
        &self,
        _undo: &[crate::journal::Change],
        _cx: &mut crate::improvement::ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        Ok(())
    }
}

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

#[test]
fn binary_on_path_is_present() {
    let runner = MockRunner::new().with_binary("clang");
    let pm = Apt;
    let dep = Dependency::new(
        DependencyKind::Binary {
            name: "clang",
            provided_by: PackageSpec::uniform("clang", 900_000_000),
        },
        "C compiler",
        "builds BPF",
    );
    let step: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.a", vec![dep]));

    let report = resolve_dependencies(&[step.as_ref()], &facts(), &runner, &pm);

    assert_eq!(report.dependencies.len(), 1);
    assert_eq!(report.dependencies[0].status, DependencyStatus::Present);
    assert!(!report.needs_install());
}

#[test]
fn missing_binary_with_available_package_is_missing() {
    let runner =
        MockRunner::new().answering("apt-cache show clang", "Package: clang\nVersion: 18.0");
    let pm = Apt;
    let dep = Dependency::new(
        DependencyKind::Binary {
            name: "clang",
            provided_by: PackageSpec::uniform("clang", 900_000_000),
        },
        "C compiler",
        "builds BPF",
    );
    let step: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.a", vec![dep]));

    let report = resolve_dependencies(&[step.as_ref()], &facts(), &runner, &pm);

    assert_eq!(report.dependencies[0].status, DependencyStatus::Missing);
    assert!(report.needs_install());
    assert_eq!(report.total_install_bytes, 900_000_000);
}

#[test]
fn unavailable_package_blocks_step() {
    let runner = MockRunner::new();
    let pm = Apt;
    let dep = Dependency::new(
        DependencyKind::Package {
            spec: PackageSpec {
                pacman: Some("missing-package"),
                apt: Some("missing-package"),
                dnf: Some("missing-package"),
                approx_bytes: 10_000,
            },
        },
        "a package this distro does not ship",
        "nothing on this distro provides it",
    );
    let step: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.a", vec![dep]));

    let report = resolve_dependencies(&[step.as_ref()], &facts(), &runner, &pm);

    assert_eq!(report.dependencies[0].status, DependencyStatus::Unavailable);
    assert_eq!(
        report.blocked_steps(),
        vec![ImprovementId::from_static("test.a")]
    );
}

#[test]
fn duplicate_dependency_across_steps_is_probed_once() {
    let runner = MockRunner::new().with_binary("clang");
    let pm = Apt;
    let dep = Dependency::new(
        DependencyKind::Binary {
            name: "clang",
            provided_by: PackageSpec::uniform("clang", 900_000_000),
        },
        "C compiler",
        "builds BPF",
    );
    let step_a: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.a", vec![dep.clone()]));
    let step_b: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.b", vec![dep]));

    let report = resolve_dependencies(&[step_a.as_ref(), step_b.as_ref()], &facts(), &runner, &pm);

    assert_eq!(report.dependencies.len(), 1);
    assert_eq!(report.dependencies[0].wanted_by.len(), 2);
}

#[test]
fn kernel_version_too_low_is_unavailable() {
    let runner = MockRunner::new();
    let pm = Apt;
    let dep = Dependency::new(
        DependencyKind::Kernel {
            min: crate::improvement::KernelVersion::new(99, 0, 0),
        },
        "future kernel",
        "needs features from kernel 99",
    );
    let step: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.a", vec![dep]));

    let report = resolve_dependencies(&[step.as_ref()], &facts(), &runner, &pm);

    assert_eq!(report.dependencies[0].status, DependencyStatus::Unavailable);
}

#[test]
fn packages_to_install_returns_distro_names() {
    let runner =
        MockRunner::new().answering("apt-cache show mangohud", "Package: mangohud\nVersion: 0.8");
    let pm = Apt;
    let dep = Dependency::new(
        DependencyKind::Package {
            spec: PackageSpec::uniform("mangohud", 5_000_000),
        },
        "overlay",
        "shows FPS",
    );
    let step: Box<dyn CoreImprovement> = Box::new(StepWithDeps::new("test.a", vec![dep]));

    let report = resolve_dependencies(&[step.as_ref()], &facts(), &runner, &pm);

    let names = report.packages_to_install(PackageManagerKind::Apt);
    assert_eq!(names, vec!["mangohud"]);
}
