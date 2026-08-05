//! Errors that stop a run before or between steps.

use thiserror::Error;

use crate::journal::JournalError;
use crate::pkg::PackageError;

/// Why a run could not proceed.
///
/// Distinct from `StepError`: that says one step failed and the run continues,
/// this says the run itself cannot go on. Failing to write the journal is here
/// because a mutation whose undo record cannot be stored must not happen.
#[derive(Debug, Error)]
pub enum RunError {
    #[error(transparent)]
    Journal(#[from] JournalError),

    #[error(transparent)]
    Package(#[from] PackageError),

    #[error(transparent)]
    Steam(#[from] crate::steam::SteamError),

    #[error("no step matched `{requested}`")]
    UnknownStep { requested: String },

    /// A dry run reached the apply phase, which it must never do.
    #[error("internal: a dry run reached the apply phase")]
    DryRunReachedApply,
}
