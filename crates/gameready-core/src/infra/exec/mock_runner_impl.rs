//! The [`CommandRunner`] implementation for [`MockRunner`].
//!
//! Split from the builders so each file stays readable: one side is the fake
//! system a test describes, the other is how a step sees it.

use std::path::{Path, PathBuf};

use crate::exec::{Cmd, CmdOutput, CommandRunner, ExecError};
use crate::improvement::Privilege;

use super::mock_runner::{MockRunner, POISONED};

impl CommandRunner for MockRunner {
    fn run(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        let output = self.record(cmd)?;
        if output.code == 0 {
            return Ok(output);
        }
        Err(ExecError::NonZeroExit {
            command: cmd.to_string(),
            code: output.code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn run_allowing_failure(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        self.record(cmd)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, ExecError> {
        self.file(path).ok_or_else(|| ExecError::Read {
            path: path.to_path_buf(),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        })
    }

    fn write_file(
        &self,
        path: &Path,
        contents: &str,
        _privilege: Privilege,
    ) -> Result<(), ExecError> {
        match self.state.lock() {
            Ok(mut state) => {
                state.files.insert(path.to_path_buf(), contents.to_owned());
                Ok(())
            }
            Err(_) => Err(ExecError::Write {
                path: path.to_path_buf(),
                source: std::io::Error::other(POISONED),
            }),
        }
    }

    fn write_private_file(&self, path: &Path, contents: &str) -> Result<(), ExecError> {
        self.write_file(path, contents, Privilege::User)
    }

    fn remove_file(&self, path: &Path, _privilege: Privilege) -> Result<(), ExecError> {
        match self.state.lock() {
            Ok(mut state) => {
                state.files.remove(path);
                Ok(())
            }
            Err(_) => Err(ExecError::Write {
                path: path.to_path_buf(),
                source: std::io::Error::other(POISONED),
            }),
        }
    }

    fn path_exists(&self, path: &Path) -> bool {
        self.file(path).is_some()
    }

    fn which(&self, binary: &str) -> Option<PathBuf> {
        self.binaries
            .contains(binary)
            .then(|| PathBuf::from("/usr/bin").join(binary))
    }
}

#[cfg(test)]
#[path = "mock_runner_test.rs"]
mod mock_runner_test;
