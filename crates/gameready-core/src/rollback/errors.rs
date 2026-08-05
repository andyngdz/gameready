//! Errors from undoing a run.

use thiserror::Error;

use crate::journal::JournalError;

/// Why a rollback could not proceed.
#[derive(Debug, Error)]
pub enum RollbackError {
    #[error(transparent)]
    Journal(#[from] JournalError),

    #[error("no run with id `{run}` in the journal")]
    UnknownRun { run: String },

    #[error("`{requested}` is not a run id")]
    MalformedRun { requested: String },

    #[error("the journal has no runs to undo")]
    NothingRecorded,
}
