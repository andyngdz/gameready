//! Errors from reading and writing the undo journal.

use std::path::PathBuf;

use thiserror::Error;

/// Why the journal could not be read or extended.
///
/// Every variant here is fatal to a run in progress. The journal is what makes
/// a change undoable, so failing to write it means the mutation it describes
/// must not happen.
#[derive(Debug, Error)]
pub enum JournalError {
    #[error("could not open journal at `{path}`")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not append to journal at `{path}`")]
    Append {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// The append reached the page cache but not the disk. Treated as a failure
    /// rather than a warning: an unsynced undo record does not survive the
    /// power loss it exists to protect against.
    #[error("could not flush journal at `{path}` to disk")]
    Sync {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("journal at `{path}` line {line} is not valid json")]
    Corrupt {
        path: PathBuf,
        line: usize,
        #[source]
        source: serde_json::Error,
    },

    #[error("could not create state directory `{path}`")]
    StateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("could not determine a state directory for the current user")]
    NoStateDir,

    #[error("could not copy `{path}` into the backup store")]
    Backup {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("no run with id `{run}` in the journal")]
    UnknownRun { run: String },
}
