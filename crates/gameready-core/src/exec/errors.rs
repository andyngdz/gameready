//! Errors from running commands and touching the filesystem.

use std::path::PathBuf;

use thiserror::Error;

/// Why a command or a filesystem operation did not succeed.
#[derive(Debug, Error)]
pub enum ExecError {
    #[error("`{command}` exited with status {code}")]
    NonZeroExit {
        command: String,
        code: i32,
        stdout: String,
        stderr: String,
    },

    #[error("`{command}` could not be started")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },

    #[error("`{command}` was killed by a signal")]
    Signalled { command: String },

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

    /// No usable way to become root was found. Carries what was looked for so
    /// the message can name it rather than saying "sudo failed" on a machine
    /// that never had sudo.
    #[error("no privilege escalation available; looked for: {looked_for}")]
    NoEscalator { looked_for: String },

    /// Escalation exists but will not authenticate without a prompt, and the
    /// run is non-interactive.
    #[error("{escalator} needs a password but the run is non-interactive")]
    EscalationNeedsPassword { escalator: String },

    /// A dry run was asked to perform a mutation. This is a programming error
    /// rather than a user-facing one: the executor should never route a
    /// mutation to the dry runner.
    #[error("dry run cannot perform `{operation}`")]
    DryRunMutation { operation: String },
}

impl ExecError {
    /// Whether retrying could plausibly succeed. Used to decide between failing
    /// the step outright and offering the user a retry.
    #[must_use]
    pub const fn is_transient(&self) -> bool {
        matches!(self, Self::Signalled { .. })
    }
}
