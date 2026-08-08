//! The [`CommandRunner`] implementation for [`MockRunner`].
//!
//! Split from the builders so each file stays readable: one side is the fake
//! system a test describes, the other is how a step sees it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::exec::{Cmd, CmdOutput, CommandRunner, ExecError};
use crate::improvement::Privilege;

use super::mock_runner::{MockRunner, POISONED};
use crate::infra::exec::constants::FAKE_BIN_DIR;

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

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ExecError> {
        // The fake system stores files, not directories, so a listing is the
        // set of immediate children of the seeded file paths that sit under
        // `path`. Seeding /sys/block/nvme0n1/queue/scheduler makes read_dir of
        // /sys/block answer with /sys/block/nvme0n1, the same shape sysfs has.
        let state = self.state.lock().map_err(|_| ExecError::Read {
            path: path.to_path_buf(),
            source: std::io::Error::other(POISONED),
        })?;
        let mut children = BTreeSet::new();
        for file in state.files.keys() {
            if let Ok(rest) = file.strip_prefix(path) {
                if let Some(first) = rest.components().next() {
                    children.insert(path.join(first.as_os_str()));
                }
            }
        }
        Ok(children.into_iter().collect())
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

    fn write_sysfs(
        &self,
        path: &Path,
        value: &str,
        _privilege: Privilege,
    ) -> Result<(), ExecError> {
        // Modeled as a privileged command so the fail-at sweep and the command
        // log see it the way apply's other mutations are seen, then the value
        // lands in the fake file so verify can read it back.
        let cmd = Cmd::root("tee").arg(path.to_string_lossy().into_owned());
        let output = self.record(&cmd)?;
        if output.code != 0 {
            return Err(ExecError::NonZeroExit {
                command: cmd.to_string(),
                code: output.code,
                stdout: output.stdout,
                stderr: output.stderr,
            });
        }
        match self.state.lock() {
            Ok(mut state) => {
                state.files.insert(path.to_path_buf(), value.to_owned());
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
        let present = self
            .state
            .lock()
            .is_ok_and(|state| state.binaries.contains(binary));
        present.then(|| PathBuf::from(FAKE_BIN_DIR).join(binary))
    }
}

#[cfg(test)]
#[path = "mock_runner_test.rs"]
mod mock_runner_test;
