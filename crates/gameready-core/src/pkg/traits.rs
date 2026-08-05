//! Driving a system's package manager.

use crate::exec::CommandRunner;
use crate::facts::PackageManagerKind;
use crate::pkg::domain::{InstallOutcome, PackageState};
use crate::pkg::errors::PackageError;

/// Installs and inspects packages through one distribution's tooling.
///
/// Hand-rolled rather than taken from a crate. Nothing maintained abstracts
/// pacman, apt, and dnf, and the operations gameready needs are not the ones a
/// general abstraction would offer: which packages a transaction *newly*
/// installed matters for the undo record, and whether a name exists in the
/// configured repositories decides whether a step is applicable at all.
pub trait PackageManager: Send + Sync {
    /// Which tooling this drives.
    fn kind(&self) -> PackageManagerKind;

    /// Whether a package is installed, available, or absent from every
    /// configured repository.
    ///
    /// Probes one name at a time. Batching would be faster, but the per-package
    /// answer is what a step needs to report why it does not apply.
    fn state(
        &self,
        runner: &dyn CommandRunner,
        package: &str,
    ) -> Result<PackageState, PackageError>;

    /// Installs the given packages in one transaction.
    ///
    /// Returns which ones were newly installed, which is the only set that
    /// removal should ever consider: a package that was already present was not
    /// put there by gameready and is not gameready's to take away.
    ///
    /// Non-interactive. A package manager that stops to ask a question under a
    /// progress display is a hang the user cannot see the cause of.
    fn install(
        &self,
        runner: &dyn CommandRunner,
        packages: &[String],
    ) -> Result<InstallOutcome, PackageError>;
}
