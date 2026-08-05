//! What gameready knows about the machine it is running on.

use serde::{Deserialize, Serialize};

use crate::improvement::KernelVersion;

/// Everything probed about the system before any step runs.
///
/// Probed once at startup and passed to every step by reference, so no step
/// reads the system directly to decide whether it applies. That keeps step
/// logic testable against a fixture instead of a live machine.
///
/// M1 carries only the kernel; distro, package manager, CPU, GPU, block
/// devices, memory, and installed tooling land in M2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemFacts {
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
    pub const fn new(kernel: KernelVersion, kernel_release: String) -> Self {
        Self {
            kernel,
            kernel_release,
        }
    }
}
