//! Answers reads from the real system and refuses every write.

use std::path::{Path, PathBuf};

use crate::exec::{Cmd, CmdOutput, CommandRunner, ExecError};
use crate::improvement::Privilege;

/// The runner behind `--dry-run`.
///
/// Reads pass through to the real system so the plan is computed against the
/// machine the user actually has, rather than against an assumed one. Writes
/// return [`ExecError::DryRunMutation`], which is a programming error surfacing
/// rather than a user-facing failure: the executor is supposed to stop before
/// the apply phase in a dry run, and this is the backstop that proves it did.
#[derive(Debug)]
pub struct DryRunner<R> {
    inner: R,
}

impl<R: CommandRunner> DryRunner<R> {
    /// Wraps a runner, keeping its reads and blocking its writes.
    #[must_use]
    pub const fn new(inner: R) -> Self {
        Self { inner }
    }

    fn refuse<T>(operation: &str) -> Result<T, ExecError> {
        Err(ExecError::DryRunMutation {
            operation: operation.to_owned(),
        })
    }
}

impl<R: CommandRunner> CommandRunner for DryRunner<R> {
    fn run(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        // A privileged command in a dry run would prompt for a password to do
        // nothing, so it is refused rather than passed through. Unprivileged
        // commands are how facts get probed and must still work.
        if cmd.needs_root() {
            return Self::refuse(&cmd.to_string());
        }
        self.inner.run(cmd)
    }

    fn run_allowing_failure(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        if cmd.needs_root() {
            return Self::refuse(&cmd.to_string());
        }
        self.inner.run_allowing_failure(cmd)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, ExecError> {
        self.inner.read_to_string(path)
    }

    fn write_file(&self, path: &Path, _contents: &str, _p: Privilege) -> Result<(), ExecError> {
        Self::refuse(&format!("write {}", path.display()))
    }

    fn write_private_file(&self, path: &Path, _contents: &str) -> Result<(), ExecError> {
        Self::refuse(&format!("write {}", path.display()))
    }

    fn remove_file(&self, path: &Path, _p: Privilege) -> Result<(), ExecError> {
        Self::refuse(&format!("remove {}", path.display()))
    }

    fn path_exists(&self, path: &Path) -> bool {
        self.inner.path_exists(path)
    }

    fn which(&self, binary: &str) -> Option<PathBuf> {
        self.inner.which(binary)
    }
}
