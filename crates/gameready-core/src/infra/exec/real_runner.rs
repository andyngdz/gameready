//! Runs commands against the live system.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::exec::{Cmd, CmdOutput, CommandRunner, Escalator, ExecError};
use crate::improvement::Privilege;

/// The production [`CommandRunner`].
///
/// Holds the escalator chosen at startup rather than looking one up per
/// command, so a machine with no `sudo` fails once during pre-flight with a
/// clear message instead of failing on the first privileged step.
#[derive(Debug, Clone)]
pub struct RealRunner {
    escalator: Escalator,
}

impl RealRunner {
    /// Uses an already-detected escalator.
    #[must_use]
    pub const fn new(escalator: Escalator) -> Self {
        Self { escalator }
    }

    /// Detects an escalator from `PATH` and builds a runner around it.
    pub fn detect() -> Result<Self, ExecError> {
        let escalator = Escalator::detect(|binary| which_on_path(binary).is_some())?;
        Ok(Self::new(escalator))
    }

    /// Which escalator privileged commands go through.
    #[must_use]
    pub const fn escalator(&self) -> Escalator {
        self.escalator
    }

    fn spawn(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        let (program, args) = if cmd.needs_root() {
            self.escalator.wrap(cmd.program(), cmd.arguments())
        } else {
            (cmd.program().to_owned(), cmd.arguments().to_vec())
        };

        let output = Command::new(&program)
            .args(&args)
            .output()
            .map_err(|source| ExecError::Spawn {
                command: cmd.to_string(),
                source,
            })?;

        let code = output.status.code().ok_or_else(|| ExecError::Signalled {
            command: cmd.to_string(),
        })?;

        Ok(CmdOutput {
            code,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

impl CommandRunner for RealRunner {
    fn run(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        let output = self.spawn(cmd)?;
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
        self.spawn(cmd)
    }

    fn read_to_string(&self, path: &Path) -> Result<String, ExecError> {
        std::fs::read_to_string(path).map_err(|source| ExecError::Read {
            path: path.to_path_buf(),
            source,
        })
    }

    fn write_file(
        &self,
        path: &Path,
        contents: &str,
        privilege: Privilege,
    ) -> Result<(), ExecError> {
        match privilege {
            Privilege::User => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| ExecError::Write {
                        path: parent.to_path_buf(),
                        source,
                    })?;
                }
                std::fs::write(path, contents).map_err(|source| ExecError::Write {
                    path: path.to_path_buf(),
                    source,
                })
            }
            // Staged as the user, then moved into place by one privileged
            // `install`. Piping through `sudo tee` would put the contents on a
            // command line or a shared pipe; `install` also sets the mode in
            // the same step, so the file is never briefly world-writable.
            Privilege::Root => {
                let staged = stage_temp_file(path, contents)?;
                let install = Cmd::root("install")
                    .arg("-m")
                    .arg("0644")
                    .arg(staged.to_string_lossy().into_owned())
                    .arg(path.to_string_lossy().into_owned());
                let result = self.run(&install).map(|_| ());
                let _ = std::fs::remove_file(&staged);
                result
            }
        }
    }

    fn remove_file(&self, path: &Path, privilege: Privilege) -> Result<(), ExecError> {
        match privilege {
            Privilege::User => match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                // Already gone is success: rollback must be safe to re-run
                // after a partial undo.
                Err(source) if source.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(source) => Err(ExecError::Write {
                    path: path.to_path_buf(),
                    source,
                }),
            },
            Privilege::Root => {
                let remove = Cmd::root("rm")
                    .arg("-f")
                    .arg(path.to_string_lossy().into_owned());
                self.run(&remove).map(|_| ())
            }
        }
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn which(&self, binary: &str) -> Option<PathBuf> {
        which_on_path(binary)
    }
}

/// Stages content in a temporary file next to nothing in particular, for a
/// privileged `install` to move into place.
fn stage_temp_file(destination: &Path, contents: &str) -> Result<PathBuf, ExecError> {
    let name = destination.file_name().map_or_else(
        || "gameready".to_owned(),
        |n| n.to_string_lossy().into_owned(),
    );
    let staged = std::env::temp_dir().join(format!("gameready-staged-{name}"));
    std::fs::write(&staged, contents).map_err(|source| ExecError::Write {
        path: staged.clone(),
        source,
    })?;
    Ok(staged)
}

/// Resolves an executable by walking `PATH`.
///
/// Hand-rolled rather than pulled in as a dependency: it is one iterator over
/// `PATH` entries, and the crates that do this add a transitive tree for it.
fn which_on_path(binary: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|candidate| candidate.is_file())
}
