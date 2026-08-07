//! An in-memory system, for testing steps without root or a real machine.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::exec::{Cmd, CmdOutput, ExecError};

/// Reported when the fake system's lock is poisoned, which only happens if a
/// test panicked while holding it. Named once so the three call sites that can
/// hit it stay in step.
pub(super) const POISONED: &str = "mock state poisoned";

/// A fake system a test can shape.
///
/// This is what makes the safety claims checkable. A step's whole lifecycle,
/// including the mutation ordering and the rollback path, runs against this
/// with no privilege and no real files, so "apply then rollback restores the
/// prior state" is an ordinary assertion rather than something only a human on
/// real hardware can confirm.
///
/// Every mutation is applied to the in-memory filesystem, so a test can assert
/// on the end state rather than only on the command sequence.
#[derive(Debug, Default)]
pub struct MockRunner {
    pub(super) state: Mutex<MockState>,
    pub(super) binaries: HashSet<String>,
    responses: HashMap<String, CmdOutput>,
    effects: HashMap<String, (PathBuf, String)>,
    failing: HashSet<String>,
    fail_at: Option<usize>,
}

/// The fake system's mutable half: what it contains and what was asked of it.
#[derive(Debug, Default)]
pub(super) struct MockState {
    pub(super) files: HashMap<PathBuf, String>,
    pub(super) commands: Vec<String>,
}

impl MockRunner {
    /// An empty system: no files, no binaries, every command succeeding with
    /// empty output.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds a file the system already has.
    #[must_use]
    pub fn with_file(self, path: impl Into<PathBuf>, contents: impl Into<String>) -> Self {
        if let Ok(mut state) = self.state.lock() {
            state.files.insert(path.into(), contents.into());
        }
        self
    }

    /// Seeds an executable that is on `PATH`.
    #[must_use]
    pub fn with_binary(mut self, name: impl Into<String>) -> Self {
        self.binaries.insert(name.into());
        self
    }

    /// Answers one command with fixed output, matched on its rendered form.
    ///
    /// The key is what [`Cmd`] displays, so a test writes the command as a user
    /// would type it rather than reconstructing the argument vector.
    #[must_use]
    pub fn answering(mut self, command: impl Into<String>, stdout: impl Into<String>) -> Self {
        self.responses.insert(
            command.into(),
            CmdOutput {
                code: 0,
                stdout: stdout.into(),
                stderr: String::new(),
            },
        );
        self
    }

    /// Makes one named command exit non-zero.
    ///
    /// Probes read meaning out of an exit status: `pacman -Q foo` failing is how
    /// "not installed" is expressed. Without this the mock answers 0 to
    /// everything unseeded, so every such probe reports the opposite of the
    /// truth and its test passes by accident.
    #[must_use]
    pub fn failing(mut self, command: impl Into<String>) -> Self {
        self.failing.insert(command.into());
        self
    }

    /// Records that a command changes a file when it runs.
    ///
    /// Without this the mock cannot model `sysctl -w`, whose whole purpose is
    /// to change what `/proc/sys/...` reads back. A step that writes a value
    /// and then verifies it would always appear to fail, so no integration test
    /// of the apply-then-verify sequence would be possible.
    #[must_use]
    pub fn where_command_writes(
        mut self,
        command: impl Into<String>,
        path: impl Into<PathBuf>,
        contents: impl Into<String>,
    ) -> Self {
        self.effects
            .insert(command.into(), (path.into(), contents.into()));
        self
    }

    /// Makes the nth command fail, counting from zero.
    ///
    /// Used to prove that a run interrupted at any point leaves a journal
    /// sufficient to roll back: the test sweeps `n` across the whole command
    /// sequence and asserts the invariant holds at every position.
    #[must_use]
    pub const fn failing_at(mut self, index: usize) -> Self {
        self.fail_at = Some(index);
        self
    }

    /// Every command run so far, in order, as rendered strings.
    #[must_use]
    pub fn commands(&self) -> Vec<String> {
        self.state
            .lock()
            .map(|state| state.commands.clone())
            .unwrap_or_default()
    }

    /// The current contents of a file in the fake system.
    #[must_use]
    pub fn file(&self, path: impl AsRef<Path>) -> Option<String> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.files.get(path.as_ref()).cloned())
    }

    /// Every path that currently exists, for asserting nothing was left behind.
    #[must_use]
    pub fn paths(&self) -> Vec<PathBuf> {
        self.state
            .lock()
            .map(|state| state.files.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub(super) fn record(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError> {
        let rendered = cmd.to_string();
        let index = match self.state.lock() {
            Ok(mut state) => {
                state.commands.push(rendered.clone());
                state.commands.len() - 1
            }
            Err(_) => {
                return Err(ExecError::Spawn {
                    command: rendered,
                    source: std::io::Error::other(POISONED),
                });
            }
        };

        if self.failing.contains(&rendered) {
            return Ok(CmdOutput {
                code: 1,
                stdout: String::new(),
                stderr: format!("{rendered}: seeded failure"),
            });
        }

        if self.fail_at == Some(index) {
            return Err(ExecError::NonZeroExit {
                command: rendered,
                code: 1,
                stdout: String::new(),
                stderr: "injected failure".to_owned(),
            });
        }

        if let Some((path, contents)) = self.effects.get(&rendered) {
            if let Ok(mut state) = self.state.lock() {
                state.files.insert(path.clone(), contents.clone());
            }
        }

        Ok(self.responses.get(&rendered).cloned().unwrap_or(CmdOutput {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        }))
    }
}
