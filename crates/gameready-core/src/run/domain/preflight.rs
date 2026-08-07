//! Types produced by the dependency resolver.

use serde::{Deserialize, Serialize};

use crate::facts::PackageManagerKind;
use crate::improvement::{Dependency, ImprovementId};
use crate::run::domain::plan::PlannedInstall;

/// One dependency after probing: present, missing but installable, or
/// unavailable on this distro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    /// The original declaration.
    pub dependency: Dependency,

    /// Every step that declared this (deduplicated by package/binary name).
    pub wanted_by: Vec<ImprovementId>,

    /// What the probe found.
    pub status: DependencyStatus,
}

/// What the resolver found for one dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyStatus {
    /// Already on the system, nothing to do.
    Present,
    /// Not installed but available in the configured repositories.
    Missing,
    /// Not available for this distro family, or kernel too old. The step that
    /// needs it becomes `NotApplicable`.
    Unavailable,
}

/// Whether the user agreed to let the run install the missing packages.
///
/// Passed in rather than decided here, because the answer belongs to whoever
/// can ask, and a run that installs on a default nobody chose is the failure
/// this type exists to make impossible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallConsent {
    /// Install what is missing, then run every step.
    Granted,
    /// Install nothing. Steps that needed a missing package are skipped; the
    /// rest of the run goes ahead.
    Declined,
}

/// Everything the resolver learned about the dependencies of a set of steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightReport {
    /// One entry per unique dependency, deduplicated.
    pub dependencies: Vec<ResolvedDependency>,

    /// Sum of `approx_bytes` for the missing (installable) dependencies.
    pub total_install_bytes: u64,
}

impl PreflightReport {
    /// Dependencies that need to be installed.
    pub fn missing(&self) -> Vec<&ResolvedDependency> {
        self.dependencies
            .iter()
            .filter(|rd| rd.status == DependencyStatus::Missing)
            .collect()
    }

    /// Dependencies already present.
    pub fn present(&self) -> Vec<&ResolvedDependency> {
        self.dependencies
            .iter()
            .filter(|rd| rd.status == DependencyStatus::Present)
            .collect()
    }

    /// Dependencies unavailable on this distro, making their steps not
    /// applicable.
    pub fn unavailable(&self) -> Vec<&ResolvedDependency> {
        self.dependencies
            .iter()
            .filter(|rd| rd.status == DependencyStatus::Unavailable)
            .collect()
    }

    /// Whether there is anything to install at all.
    #[must_use]
    pub fn needs_install(&self) -> bool {
        self.dependencies
            .iter()
            .any(|rd| rd.status == DependencyStatus::Missing)
    }

    /// Steps that must be demoted to `NotApplicable` because one of their
    /// dependencies is unavailable.
    pub fn blocked_steps(&self) -> Vec<ImprovementId> {
        self.steps_wanting(DependencyStatus::Unavailable)
    }

    /// Steps that cannot run unless something is installed first.
    ///
    /// These are the steps a declined install screen takes away, which is why
    /// the screen names them before asking.
    pub fn steps_needing_install(&self) -> Vec<ImprovementId> {
        self.steps_wanting(DependencyStatus::Missing)
    }

    fn steps_wanting(&self, status: DependencyStatus) -> Vec<ImprovementId> {
        let mut steps = Vec::new();
        for resolved in self.dependencies.iter().filter(|rd| rd.status == status) {
            for step_id in &resolved.wanted_by {
                if !steps.contains(step_id) {
                    steps.push(step_id.clone());
                }
            }
        }
        steps
    }

    /// Package names that need to be installed, resolved for this distro.
    pub fn packages_to_install(&self, pm: PackageManagerKind) -> Vec<String> {
        self.missing()
            .iter()
            .filter_map(|rd| rd.dependency.package_name(pm).map(String::from))
            .collect()
    }

    /// The missing dependencies as install lines a screen can render.
    pub fn planned_installs(&self, pm: PackageManagerKind) -> Vec<PlannedInstall> {
        self.missing()
            .iter()
            .filter_map(|rd| {
                let package = rd.dependency.package_name(pm)?.to_owned();
                Some(PlannedInstall {
                    package,
                    what: rd.dependency.what.to_owned(),
                    why: rd.dependency.why.to_owned(),
                    approx_bytes: rd.dependency.approx_bytes(),
                })
            })
            .collect()
    }
}
