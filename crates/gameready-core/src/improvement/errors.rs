//! Errors an improvement can fail with.

use std::error::Error as _;
use std::path::PathBuf;

use itertools::Itertools as _;
use thiserror::Error;

use crate::exec::ExecError;
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
    /// A command the step judged to have failed on its own terms, rather than
    /// by exiting non-zero. `Exec` covers the ordinary case; this is for a step
    /// that ran something successfully and rejected what came back.
    #[error("`{command}` exited with status {code}: {stderr}")]
    Command {
        command: String,
        code: i32,
        stderr: String,
    },

    /// A command or a filesystem operation failed. Transparent because
    /// [`ExecError`] already names the path, the command, the exit status, and
    /// the stderr; repeating any of that here would print it twice.
    #[error(transparent)]
    Exec(#[from] ExecError),

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

    /// The start command succeeded but what it started never came up.
    ///
    /// Used where the start is asynchronous, such as a systemd unit whose
    /// wrapper shell is up before the scheduler's BPF program has attached.
    /// The step waits a bounded window and fails with this rather than
    /// reporting success for work that has not happened.
    #[error("{what} did not attach within {window} s")]
    StartupTimeout { what: String, window: u64 },

    #[error("could not parse {what} from `{path}`")]
    Parse {
        what: &'static str,
        path: PathBuf,
        #[source]
        source: ParseFailure,
    },

    #[error(transparent)]
    Journal(#[from] crate::journal::JournalError),

    /// The package tooling refused a query or an install. Kept as its own
    /// variant rather than flattened into `Command`, because the caller can
    /// tell "this package does not exist here" from "the command broke".
    #[error(transparent)]
    Packages(#[from] crate::pkg::PackageError),

    /// Steam's config could not be read as the format it is meant to be in.
    /// Distinct from an IO failure: the file was readable and did not look like
    /// what was expected, so nothing was written.
    #[error(transparent)]
    SteamConfig(#[from] crate::steam::VdfError),
}

impl StepError {
    /// This failure and every cause under it, on one line.
    ///
    /// `to_string` renders only the outermost message, so a `Write` says which
    /// path could not be written and drops the reason the system gave for
    /// refusing. Every call site that turns a `StepError` into a report string
    /// uses this instead; the report is the only place a user ever sees why a
    /// step failed.
    ///
    /// One line because a command's stderr arrives with its newlines and the
    /// summary row it lands in cannot take them.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut described = one_line(&self.to_string());
        let mut cause = self.source();
        while let Some(error) = cause {
            described.push_str(": ");
            described.push_str(&one_line(&error.to_string()));
            cause = error.source();
        }
        described
    }
}

/// Collapses every run of whitespace, newlines included, to a single space.
fn one_line(text: &str) -> String {
    text.split_whitespace().join(" ")
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

#[cfg(test)]
#[path = "errors_test.rs"]
mod errors_test;
