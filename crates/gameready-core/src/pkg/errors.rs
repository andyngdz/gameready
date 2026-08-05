//! Errors from driving a package manager.

use thiserror::Error;

use crate::exec::ExecError;
use crate::facts::PackageManagerKind;

/// Why a package operation did not complete.
#[derive(Debug, Error)]
pub enum PackageError {
    #[error("could not ask {manager} about `{package}`")]
    Query {
        manager: PackageManagerKind,
        package: String,
        #[source]
        source: ExecError,
    },

    #[error("{manager} could not install {}", packages.join(", "))]
    Install {
        manager: PackageManagerKind,
        packages: Vec<String>,
        #[source]
        source: ExecError,
    },

    /// The repository metadata is too old or absent to answer a query. Distinct
    /// from a package being unavailable: refreshing may change the answer.
    #[error("{manager} has no repository metadata; refresh it first")]
    NoMetadata { manager: PackageManagerKind },
}
