//! How the fake system changes while a run is under way.
//!
//! Split from the builders because it answers a different question. The rest
//! of [`MockRunner`] describes the machine a test starts on; this describes
//! what running a command turns into something else, which is the only way to
//! test a step that probes again after an earlier step succeeded.

use crate::exec::CmdOutput;

use super::mock_runner::MockRunner;

/// What running one command changes about the rest of the fake system.
#[derive(Debug)]
pub(super) enum Unlock {
    /// A command that answered one way starts answering another.
    Answer { command: String, stdout: String },
    /// A binary that was not on `PATH` appears on it.
    Binary { name: String },
}

impl MockRunner {
    /// Records that running `trigger` changes how `command` answers.
    ///
    /// The system a run inspects is not the system it finishes on. Adding a
    /// package repository is the case this exists for: `apt-cache show lutris`
    /// fails before `add-apt-repository` and succeeds after it, and without
    /// that a test cannot tell a step that probes again from one that does not.
    #[must_use]
    pub fn where_command_changes_answer(
        mut self,
        trigger: impl Into<String>,
        command: impl Into<String>,
        stdout: impl Into<String>,
    ) -> Self {
        self.triggers
            .entry(trigger.into())
            .or_default()
            .push(Unlock::Answer {
                command: command.into(),
                stdout: stdout.into(),
            });
        self
    }

    /// Records that running `trigger` puts a binary on `PATH`.
    ///
    /// The other half of the same story: installing gamemode is what makes
    /// `which gamemoderun` start answering, and a step that probes for it
    /// before and after that install has to see two different answers.
    #[must_use]
    pub fn where_command_adds_binary(
        mut self,
        trigger: impl Into<String>,
        binary: impl Into<String>,
    ) -> Self {
        self.triggers
            .entry(trigger.into())
            .or_default()
            .push(Unlock::Binary {
                name: binary.into(),
            });
        self
    }

    /// Applies whatever `rendered` turns on, so the next command sees it.
    pub(super) fn unlock(&self, rendered: &str) {
        let Some(unlocks) = self.triggers.get(rendered) else {
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };

        for unlock in unlocks {
            match unlock {
                Unlock::Answer { command, stdout } => {
                    state.unlocked.insert(
                        command.clone(),
                        CmdOutput {
                            code: 0,
                            stdout: stdout.clone(),
                            stderr: String::new(),
                        },
                    );
                }
                Unlock::Binary { name } => {
                    state.binaries.insert(name.clone());
                }
            }
        }
    }

    /// The answer a trigger has switched on for this command, if any.
    pub(super) fn unlocked_answer(&self, rendered: &str) -> Option<CmdOutput> {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.unlocked.get(rendered).cloned())
    }
}
