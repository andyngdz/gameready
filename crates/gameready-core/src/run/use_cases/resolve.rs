//! Resolving step dependencies before any step applies.
//!
//! Collects every [`Dependency`] from the applicable steps, subtracts what the
//! system already has, and returns a [`PreflightReport`] the caller can present
//! before installing anything.

use crate::exec::CommandRunner;
use crate::facts::{PackageManagerKind, SystemFacts};
use crate::improvement::{CoreImprovement, Dependency, DependencyKind};
use crate::pkg::{PackageManager, PackageState};
use crate::run::domain::{DependencyStatus, PreflightReport, ResolvedDependency};

/// Probes every dependency declared by `steps` and classifies each one.
///
/// A dependency that appears in more than one step is probed once and attributed
/// to all steps that declared it. The caller gets a single list with no
/// duplicates, ready for the install prompt.
pub fn resolve_dependencies(
    steps: &[Box<dyn CoreImprovement>],
    facts: &SystemFacts,
    runner: &dyn CommandRunner,
    pkg_manager: &dyn PackageManager,
) -> PreflightReport {
    let mut resolved: Vec<ResolvedDependency> = Vec::new();
    let mut seen_keys: Vec<String> = Vec::new();

    for step in steps {
        for dep in step.dependencies() {
            let key = dep_key(dep, facts.distro.package_manager());
            if let Some(pos) = seen_keys.iter().position(|k| k == &key) {
                resolved[pos].wanted_by.push(step.id());
                continue;
            }

            let status = probe_one(dep, facts, runner, pkg_manager);
            seen_keys.push(key);
            resolved.push(ResolvedDependency {
                dependency: dep.clone(),
                wanted_by: vec![step.id()],
                status,
            });
        }
    }

    let total_bytes: u64 = resolved
        .iter()
        .filter(|rd| rd.status == DependencyStatus::Missing)
        .map(|rd| spec_bytes(&rd.dependency, facts.distro.package_manager()))
        .sum();

    PreflightReport {
        dependencies: resolved,
        total_install_bytes: total_bytes,
    }
}

/// Stable key for deduplication. Two deps with the same resolved package name
/// (or the same binary name, or the same feature path) are the same thing.
fn dep_key(dep: &Dependency, pm: PackageManagerKind) -> String {
    match &dep.kind {
        DependencyKind::Binary { name, .. } => format!("bin:{name}"),
        DependencyKind::Package { spec } => {
            let pkg = package_name(spec, pm).unwrap_or("?");
            format!("pkg:{pkg}")
        }
        DependencyKind::Kernel { min } => format!("kernel:{min}"),
        DependencyKind::Feature { path } => format!("feature:{path}"),
    }
}

fn spec_bytes(dep: &Dependency, _pm: PackageManagerKind) -> u64 {
    match &dep.kind {
        DependencyKind::Binary { provided_by, .. } => provided_by.approx_bytes,
        DependencyKind::Package { spec } => spec.approx_bytes,
        DependencyKind::Kernel { .. } | DependencyKind::Feature { .. } => 0,
    }
}

fn probe_one(
    dep: &Dependency,
    facts: &SystemFacts,
    runner: &dyn CommandRunner,
    pkg_manager: &dyn PackageManager,
) -> DependencyStatus {
    match &dep.kind {
        DependencyKind::Binary { name, provided_by } => {
            if runner.which(name).is_some() {
                return DependencyStatus::Present;
            }
            let pkg = match package_name(provided_by, facts.distro.package_manager()) {
                Some(name) => name,
                None => return DependencyStatus::Unavailable,
            };
            match pkg_manager.state(runner, pkg) {
                Ok(PackageState::Available) => DependencyStatus::Missing,
                Ok(PackageState::Installed { .. }) => DependencyStatus::Present,
                Ok(PackageState::Unavailable) | Err(_) => DependencyStatus::Unavailable,
            }
        }
        DependencyKind::Package { spec } => {
            let pkg = match package_name(spec, facts.distro.package_manager()) {
                Some(name) => name,
                None => return DependencyStatus::Unavailable,
            };
            match pkg_manager.state(runner, pkg) {
                Ok(PackageState::Installed { .. }) => DependencyStatus::Present,
                Ok(PackageState::Available) => DependencyStatus::Missing,
                Ok(PackageState::Unavailable) | Err(_) => DependencyStatus::Unavailable,
            }
        }
        DependencyKind::Kernel { min } => {
            if facts.kernel >= *min {
                DependencyStatus::Present
            } else {
                DependencyStatus::Unavailable
            }
        }
        DependencyKind::Feature { path } => {
            if runner.path_exists(path.as_ref()) {
                DependencyStatus::Present
            } else {
                DependencyStatus::Unavailable
            }
        }
    }
}

/// Resolves a [`PackageSpec`] to the concrete name for this distro family.
fn package_name(
    spec: &crate::improvement::PackageSpec,
    pm: PackageManagerKind,
) -> Option<&'static str> {
    match pm {
        PackageManagerKind::Pacman => spec.pacman,
        PackageManagerKind::Apt => spec.apt,
        PackageManagerKind::Dnf => spec.dnf,
    }
}

#[cfg(test)]
#[path = "resolve_test.rs"]
mod resolve_test;
