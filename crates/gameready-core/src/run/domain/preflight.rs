//! Types produced by the dependency resolver.

use serde::{Deserialize, Serialize};

use crate::facts::PackageManagerKind;
use crate::improvement::{Dependency, DependencyKind, ImprovementId};

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
        let mut blocked = Vec::new();
        for rd in &self.dependencies {
            if rd.status == DependencyStatus::Unavailable {
                for step_id in &rd.wanted_by {
                    if !blocked.contains(step_id) {
                        blocked.push(step_id.clone());
                    }
                }
            }
        }
        blocked
    }

    /// Package names that need to be installed, resolved for this distro.
    pub fn packages_to_install(&self, pm: PackageManagerKind) -> Vec<String> {
        self.missing()
            .iter()
            .filter_map(|rd| match &rd.dependency.kind {
                DependencyKind::Binary { provided_by, .. } => {
                    provided_by.name_for(pm).map(String::from)
                }
                DependencyKind::Package { spec } => spec.name_for(pm).map(String::from),
                DependencyKind::Kernel { .. } | DependencyKind::Feature { .. } => None,
            })
            .collect()
    }
}
