use std::time::Instant;

use super::*;
use crate::facts::PackageManagerKind;
use crate::improvement::{Dependency, DependencyKind, ImprovementId, PackageSpec};
use crate::run::domain::preflight::{DependencyStatus, ResolvedDependency};

fn resolved(package: &'static str, status: DependencyStatus) -> ResolvedDependency {
    ResolvedDependency {
        dependency: Dependency::new(
            DependencyKind::Package {
                spec: PackageSpec::uniform(package, 5_000_000),
            },
            "a test package",
            "so the plan has something to report",
        ),
        wanted_by: vec![ImprovementId::from_static("test.a")],
        status,
    }
}

fn plan_needing(status: DependencyStatus) -> RunPlan {
    RunPlan {
        settled: Vec::new(),
        pending: Vec::new(),
        preflight: PreflightReport {
            dependencies: vec![resolved("mangohud", status)],
            total_install_bytes: 5_000_000,
        },
        step_installs: Vec::new(),
        step_present: Vec::new(),
        started: Instant::now(),
    }
}

#[test]
fn a_plan_with_a_missing_package_needs_an_install() {
    let plan = plan_needing(DependencyStatus::Missing);

    assert!(plan.installs_anything());
    assert_eq!(
        plan.packages_to_install(PackageManagerKind::Apt),
        vec!["mangohud"]
    );
}

#[test]
fn a_plan_whose_packages_are_present_asks_for_nothing() {
    let plan = plan_needing(DependencyStatus::Present);

    assert!(!plan.installs_anything());
    assert!(plan.packages_to_install(PackageManagerKind::Apt).is_empty());
}

#[test]
fn a_plan_with_no_pending_steps_is_empty() {
    assert!(plan_needing(DependencyStatus::Missing).is_empty());
}
