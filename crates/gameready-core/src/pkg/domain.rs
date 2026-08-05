//! What is known about a package, and what installing it did.

use serde::{Deserialize, Serialize};

/// Whether a package is present, obtainable, or neither.
///
/// The third case is the interesting one. `scx-scheds` does not exist in the
/// Ubuntu archive at all, so a step needing it is not failing, it is not
/// applicable, and the difference is what the user reads on the summary screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum PackageState {
    /// Already on the system. `version` is what the package manager reported,
    /// when it reported one.
    Installed { version: Option<String> },

    /// Not installed, but present in a configured repository.
    Available,

    /// Not in any configured repository, so no amount of installing will help.
    Unavailable,
}

impl PackageState {
    /// Whether installing this would do anything.
    #[must_use]
    pub const fn needs_install(&self) -> bool {
        matches!(self, Self::Available)
    }

    /// Whether the package can be obtained at all on this system.
    #[must_use]
    pub const fn is_obtainable(&self) -> bool {
        matches!(self, Self::Installed { .. } | Self::Available)
    }
}

/// What one install transaction changed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallOutcome {
    /// Everything the caller asked for.
    pub requested: Vec<String>,

    /// The subset that was not already present.
    ///
    /// The undo record stores this rather than `requested`: a package that was
    /// already installed was not put there by gameready, so removing it would
    /// take away something the user had before the run.
    pub newly_installed: Vec<String>,
}

impl InstallOutcome {
    /// Whether the transaction changed anything.
    #[must_use]
    pub fn changed_anything(&self) -> bool {
        !self.newly_installed.is_empty()
    }
}

#[cfg(test)]
#[path = "domain_test.rs"]
mod domain_test;
