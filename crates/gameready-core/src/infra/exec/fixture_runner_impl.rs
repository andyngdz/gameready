//! What the fixture machine does when a step asks it something.

use std::path::{Path, PathBuf};

use crate::exec::{Cmd, CmdOutput, CommandRunner, ExecError};
use crate::improvement::Privilege;
use crate::infra::exec::constants::FAKE_BIN_DIR;
use crate::infra::exec::fixture_runner::FixtureRunner;

impl FixtureRunner {
    /// Reports a write as the mistake it is.
    ///
    /// A fixture stands in for someone else's machine, so a run that reached a
    /// write has left the ground the snapshot covers. Failing here names the
    /// operation rather than letting the run continue against a machine that
    /// silently did not change.
    fn refuse<T>(operation: &str) -> Result<T, ExecError> {
        Err(ExecError::DryRunMutation {
            operation: operation.to_owned(),
        })
    }
}

impl CommandRunner for FixtureRunner {
    fn run(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        let rendered = cmd.to_string();
        let output = self.answer(&rendered);
        if output.code == 0 {
            return Ok(output);
        }

        Err(ExecError::NonZeroExit {
            command: rendered,
            code: output.code,
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    fn run_allowing_failure(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        Ok(self.answer(&cmd.to_string()))
    }

    fn read_to_string(&self, path: &Path) -> Result<String, ExecError> {
        let resolved = self.resolve(path);
        std::fs::read_to_string(&resolved).map_err(|source| ExecError::Read {
            // The path the caller asked for, not the one inside the fixture: an
            // error message naming a fixture directory would read as a bug in
            // gameready rather than as a gap in the fake machine.
            path: path.to_path_buf(),
            source,
        })
    }

    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ExecError> {
        let resolved = self.resolve(path);
        let listing = std::fs::read_dir(&resolved).map_err(|source| ExecError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        // Mapped back out of the fixture so a step sees the paths it would see
        // on a real machine, and sorted so the listing does not depend on the
        // order the filesystem happens to hand them over in.
        let mut entries: Vec<PathBuf> = listing
            .flatten()
            .filter_map(|entry| {
                entry
                    .path()
                    .strip_prefix(&resolved)
                    .ok()
                    .map(Path::to_path_buf)
            })
            .map(|name| path.join(name))
            .collect();
        entries.sort();
        Ok(entries)
    }

    fn write_file(&self, path: &Path, _contents: &str, _p: Privilege) -> Result<(), ExecError> {
        Self::refuse(&format!("write {}", path.display()))
    }

    fn write_sysfs(&self, path: &Path, _value: &str, _p: Privilege) -> Result<(), ExecError> {
        Self::refuse(&format!("write {}", path.display()))
    }

    fn remove_file(&self, path: &Path, _p: Privilege) -> Result<(), ExecError> {
        Self::refuse(&format!("remove {}", path.display()))
    }

    fn path_exists(&self, path: &Path) -> bool {
        self.resolve(path).exists()
    }

    /// Refused, like every other write.
    ///
    /// A fixture stands in for a machine. One that reached the network anyway
    /// would make every screen taken against it a screen of something else.
    fn download(&self, url: &str, _dest: &Path, _on_bytes: &dyn Fn(u64)) -> Result<(), ExecError> {
        Err(ExecError::Download {
            url: url.to_owned(),
            detail: "this run reads a fixture directory and reaches no network".to_owned(),
        })
    }

    fn which(&self, binary: &str) -> Option<PathBuf> {
        self.has_binary(binary)
            .then(|| PathBuf::from(FAKE_BIN_DIR).join(binary))
    }
}

#[cfg(test)]
#[path = "fixture_runner_impl_test.rs"]
mod fixture_runner_impl_test;
