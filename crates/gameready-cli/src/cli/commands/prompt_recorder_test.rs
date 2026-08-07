use std::cell::RefCell;

use anyhow::Result;
use gameready_core::infra::exec::MockRunner;

/// The `sudo ` prefix `Cmd::Display` writes for any command needing root, which
/// is how a recorded command line says it was privileged.
const PRIVILEGED: &str = "sudo ";

/// Stands in for the password prompt, and remembers what the machine had
/// already been asked to run when it fired.
///
/// Asserting only that the prompt happened would pass even if it happened after
/// the first privileged command, which is the bug this guards.
pub struct PromptRecorder<'a> {
    runner: &'a MockRunner,
    calls: RefCell<Vec<Vec<String>>>,
}

impl<'a> PromptRecorder<'a> {
    pub fn new(runner: &'a MockRunner) -> Self {
        Self {
            runner,
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Answers the prompt, recording the command log as it stood.
    pub fn answer(&self) -> Result<()> {
        self.calls.borrow_mut().push(self.runner.commands());
        Ok(())
    }

    pub fn times_asked(&self) -> usize {
        self.calls.borrow().len()
    }

    /// Whether any privileged command had already run when the prompt fired.
    pub fn ran_anything_privileged_first(&self) -> bool {
        self.calls
            .borrow()
            .first()
            .is_some_and(|before| before.iter().any(|line| line.starts_with(PRIVILEGED)))
    }

    /// Whether the run reached a privileged command at all. Without this the
    /// ordering assertion would pass on a run that never needed root.
    pub fn reached_a_privileged_command(&self) -> bool {
        self.runner
            .commands()
            .iter()
            .any(|line| line.starts_with(PRIVILEGED))
    }
}
