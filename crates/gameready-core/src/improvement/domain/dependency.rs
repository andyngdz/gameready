//! What a step needs present before it can run.
//!
//! The executor collects these from every selected step, subtracts what the
//! system already has, and installs the remainder in one transaction before any
//! step applies. A step therefore never discovers a missing toolchain halfway
//! through and dies with the system half-changed.

use serde::{Deserialize, Serialize};

use crate::facts::PackageManagerKind;

/// One prerequisite, with the text shown to the user before anything is
/// installed on their machine.
///
/// `what` and `why` are not optional. The pre-flight screen is the only place a
/// user sees what is about to land on their system, so a line that reads
/// `clang  ~900MB` and nothing else is not good enough.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub kind: DependencyKind,

    /// What the thing is, for someone who has never heard of it. One sentence.
    /// "the LLVM C and C++ compiler"
    pub what: &'static str,

    /// Why this step needs it. One sentence.
    /// "the bundle ships a BPF program, which only clang compiles"
    pub why: &'static str,
}

impl Dependency {
    #[must_use]
    pub const fn new(kind: DependencyKind, what: &'static str, why: &'static str) -> Self {
        Self { kind, what, why }
    }

    /// Whether the executor can resolve this by installing something. A kernel
    /// version requirement cannot be installed, so a step missing one becomes
    /// `NotApplicable` instead of entering the install set.
    #[must_use]
    pub const fn is_installable(&self) -> bool {
        matches!(
            self.kind,
            DependencyKind::Binary { .. } | DependencyKind::Package { .. }
        )
    }

    /// Rough installed size, or zero when this is not a package at all.
    ///
    /// One number across all three families: the sizes are hand-written
    /// estimates, and pretending they differ per distro would be false
    /// accuracy.
    #[must_use]
    pub const fn approx_bytes(&self) -> u64 {
        match &self.kind {
            DependencyKind::Binary { provided_by, .. } => provided_by.approx_bytes,
            DependencyKind::Package { spec } => spec.approx_bytes,
            DependencyKind::Kernel { .. } => 0,
        }
    }

    /// What this is called on the given package tooling.
    ///
    /// `None` for a kernel version, which no package manager has a name for.
    #[must_use]
    pub const fn package_name(&self, packages: PackageManagerKind) -> Option<&'static str> {
        match &self.kind {
            DependencyKind::Binary { provided_by, .. } => provided_by.name_for(packages),
            DependencyKind::Package { spec } => spec.name_for(packages),
            DependencyKind::Kernel { .. } => None,
        }
    }
}

/// The kinds of prerequisite a step can declare. Split by how each one is
/// probed and whether the executor can do anything about it being absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DependencyKind {
    /// An executable that must be on `PATH`, and the package that provides it.
    /// Probed by looking up the name rather than asking the package manager,
    /// because a user may have installed it outside the package manager.
    Binary {
        name: &'static str,
        provided_by: PackageSpec,
    },

    /// A package that provides no executable of its own, such as headers.
    /// Probed by asking the package manager directly.
    Package { spec: PackageSpec },

    /// Minimum kernel version. Not installable.
    Kernel { min: KernelVersion },
}

/// Package names differ per distro family, so a spec carries one name per
/// family rather than assuming a shared name. `None` means the package does not
/// exist for that family and any step requiring it is `NotApplicable` there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSpec {
    pub pacman: Option<&'static str>,
    pub apt: Option<&'static str>,
    pub dnf: Option<&'static str>,

    /// Rough installed size, shown on the pre-flight screen so the user can see
    /// that clang is 900MB and mangohud is 5MB before agreeing to both.
    pub approx_bytes: u64,
}

impl PackageSpec {
    /// The common case: one name across all three families.
    #[must_use]
    pub const fn uniform(name: &'static str, approx_bytes: u64) -> Self {
        Self {
            pacman: Some(name),
            apt: Some(name),
            dnf: Some(name),
            approx_bytes,
        }
    }

    /// What this package is called on the given tooling.
    ///
    /// `None` means it does not exist there at all, which is a different answer
    /// from "exists but is not in a configured repository": the first is known
    /// without asking the system, the second needs a query.
    #[must_use]
    pub const fn name_for(&self, pm: PackageManagerKind) -> Option<&'static str> {
        match pm {
            PackageManagerKind::Pacman => self.pacman,
            PackageManagerKind::Apt => self.apt,
            PackageManagerKind::Dnf => self.dnf,
        }
    }
}

/// A kernel version comparable against `uname -r`. Kept as three numbers rather
/// than a `semver::Version` because kernel releases carry suffixes like
/// `7.0.0-29-generic` that are not semver prerelease tags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KernelVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl KernelVersion {
    #[must_use]
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl std::fmt::Display for KernelVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
