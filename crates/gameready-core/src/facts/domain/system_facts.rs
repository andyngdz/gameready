//! What gameready knows about the machine it is running on.

use serde::{Deserialize, Serialize};

use crate::facts::domain::distro::Distro;
use crate::improvement::KernelVersion;

/// Everything probed about the system before any step runs.
///
/// Probed once at startup and passed to every step by reference, so no step
/// reads the system directly to decide whether it applies. That keeps step
/// logic testable against a fixture instead of a live machine.
///
/// CPU, GPU, block devices, memory, and installed tooling land later; the
/// kernel and the distro are what the first steps need.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemFacts {
    /// Which distribution this is and what installs software on it.
    pub distro: Distro,

    /// Running kernel, parsed from `uname -r`.
    pub kernel: KernelVersion,

    /// The raw `uname -r` string, kept because distro kernels carry suffixes
    /// like `7.0.0-29-generic` that the parsed version drops and that a user
    /// needs to see to recognise their own machine.
    pub kernel_release: String,
}

impl SystemFacts {
    /// Builds facts from already-probed values.
    #[must_use]
    pub const fn new(distro: Distro, kernel: KernelVersion, kernel_release: String) -> Self {
        Self {
            distro,
            kernel,
            kernel_release,
        }
    }
}

#[cfg(any(test, feature = "testkit"))]
impl SystemFacts {
    /// A stand-in machine for tests: this laptop's kernel, on the given family.
    ///
    /// Steps read facts to decide whether they apply, so nearly every test
    /// needs one. Building it by hand in each test would mean editing dozens of
    /// call sites every time a field is added.
    #[must_use]
    pub fn fixture(family: crate::facts::Family) -> Self {
        Self::new(
            Distro {
                id: family.to_string(),
                name: format!("{family} (fixture)"),
                version_id: None,
                family,
                root_filesystem: crate::facts::RootFilesystem::Mutable,
            },
            KernelVersion::new(7, 0, 0),
            "7.0.0-29-generic".to_owned(),
        )
    }
}
