//! Errors an improvement can fail with.

use std::path::PathBuf;

use thiserror::Error;

use crate::improvement::domain::ImprovementId;

/// Rejects an id built at runtime before it can reach the journal.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ImprovementIdError {
    #[error("improvement id is empty")]
    Empty,

    #[error("improvement id `{id}` has an empty segment")]
    EmptySegment { id: String },

    #[error(
        "improvement id `{id}` has segment `{segment}`; \
         segments must be lowercase ascii, digits, or `-`"
    )]
    Malformed { id: String, segment: String },
}

/// Why a step could not complete. Callers branch on these to decide whether to
/// retry, skip the rest of the run, or roll back, so each variant carries the
/// context that decision needs rather than a flattened message.
#[derive(Debug, Error)]
pub enum StepError {
    #[error("reading `{path}` failed")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("writing `{path}` failed")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("`{command}` exited with status {code}")]
    Command {
        command: String,
        code: i32,
        stderr: String,
    },

    #[error("`{command}` could not be started")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },

    /// The change was made but reading it back did not show the expected value.
    /// The executor rolls the step back on this rather than reporting success.
    #[error("{step} applied but verification failed: {failed} of {total} checks did not pass")]
    VerificationFailed {
        step: ImprovementId,
        failed: usize,
        total: usize,
    },

    /// A precondition the step declared was not met at apply time. Probing
    /// should normally catch this first; reaching it means the system changed
    /// between probe and apply.
    #[error("{step} precondition no longer holds: {detail}")]
    PreconditionLost { step: ImprovementId, detail: String },

    #[error("could not parse {what} from `{path}`")]
    Parse {
        what: &'static str,
        path: PathBuf,
        #[source]
        source: ParseFailure,
    },

    #[error(transparent)]
    Journal(#[from] crate::journal::JournalError),
}

/// The concrete ways reading a system file can fail to yield a usable value.
/// Kept as its own enum so `StepError::Parse` names a real type: a boxed
/// `dyn Error` here would make every caller stringify to learn anything.
#[derive(Debug, Error)]
pub enum ParseFailure {
    #[error(transparent)]
    Integer(#[from] std::num::ParseIntError),

    #[error("expected {expected}, found `{found}`")]
    Unexpected {
        expected: &'static str,
        found: String,
    },
}
