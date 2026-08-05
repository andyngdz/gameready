//! Implementations of [`crate::pkg::PackageManager`].

mod apt;
mod dnf;
mod pacman;

use crate::exec::CommandRunner;
use crate::facts::PackageManagerKind;
use crate::pkg::{PackageError, PackageManager, PackageState};

pub use apt::Apt;
pub use dnf::Dnf;
pub use pacman::Pacman;

/// Builds the manager for a family's tooling.
#[must_use]
pub fn for_kind(kind: PackageManagerKind) -> Box<dyn PackageManager> {
    match kind {
        PackageManagerKind::Pacman => Box::new(Pacman),
        PackageManagerKind::Apt => Box::new(Apt),
        PackageManagerKind::Dnf => Box::new(Dnf),
    }
}

/// Filters a request down to the packages that are not already installed.
///
/// Shared by all three implementations because the reasoning is the same
/// everywhere: only what a run newly installs belongs in the undo record, and a
/// package the user already had must not be removable by a rollback.
///
/// An unavailable package is dropped rather than raised. The caller has already
/// decided the step is applicable, and a repository that lost the package
/// between probe and install should not fail the whole transaction.
fn newly_installed(
    manager: &dyn PackageManager,
    runner: &dyn CommandRunner,
    packages: &[String],
) -> Result<Vec<String>, PackageError> {
    let mut pending = Vec::new();
    for package in packages {
        if manager.state(runner, package)? == PackageState::Available {
            pending.push(package.clone());
        }
    }
    Ok(pending)
}
