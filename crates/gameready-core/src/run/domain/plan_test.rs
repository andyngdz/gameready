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
        deferred: Vec::new(),
        preflight: PreflightReport {
            dependencies: vec![resolved("mangohud", status)],
            total_install_bytes: 5_000_000,
        },
        step_installs: Vec::new(),
        step_present: Vec::new(),
        started: Instant::now(),
    }
}

/// A plan whose pending steps are the catalog steps with these ids.
///
/// Real steps rather than a fake, because the question is what a step's own
/// `privilege` says, and a fake that answers it is a test of the fake.
fn plan_applying(ids: &[&str]) -> RunPlan {
    let mut plan = plan_needing(DependencyStatus::Present);
    plan.pending = crate::steps::core_steps()
        .into_iter()
        .filter(|step| ids.contains(&step.id().as_str()))
        .collect();
    assert_eq!(plan.pending.len(), ids.len(), "an id named no catalog step");
    plan
}

#[test]
fn a_plan_of_nothing_but_the_users_own_files_needs_no_password() {
    // Reading systemd unit states and writing Steam's config are both the
    // user's own business, and asking for a password to do them teaches a user
    // to type it without reading what asked.
    let plan = plan_applying(&["core.conflicts", "core.proton.ge"]);

    assert!(!plan.needs_root());
}

#[test]
fn one_step_that_reaches_outside_the_home_is_enough_to_need_a_password() {
    let plan = plan_applying(&["core.conflicts", "core.sysctl.max-map-count"]);

    assert!(plan.needs_root());
}

#[test]
fn a_held_open_step_counts_towards_the_password() {
    // It may be released mid-run, and stopping to ask then would be asking
    // after the run had already started changing things.
    let mut plan = plan_applying(&["core.conflicts"]);
    plan.deferred = vec![Deferred {
        step: crate::steps::core_steps()
            .into_iter()
            .find(|step| step.id().as_str() == "core.sysctl.max-map-count")
            .expect("a catalog step"),
        reason: "held open for the test".to_owned(),
        waiting_on: vec![ImprovementId::from_static("core.conflicts")],
    }];

    assert!(plan.needs_root());
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
