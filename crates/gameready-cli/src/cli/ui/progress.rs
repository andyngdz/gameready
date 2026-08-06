//! Live spinner on stderr while steps run.
//!
//! Shows a spinner during probing and applying so the user always sees
//! something happening. Clears itself when the run finishes, leaving the
//! summary as the only thing on screen.

use gameready_core::run::RunEvent;
use indicatif::ProgressBar;

/// A single-line spinner for whatever the executor is doing right now.
pub struct ProgressView {
    spinner: Option<ProgressBar>,
}

impl ProgressView {
    #[must_use]
    pub fn new() -> Self {
        Self { spinner: None }
    }

    /// Handles one event from the executor.
    pub fn on_event(&mut self, event: RunEvent) {
        match event {
            RunEvent::Probing { .. } => {
                if self.spinner.is_none() {
                    self.start("Checking system...".to_owned());
                }
            }

            RunEvent::Planned { .. } => self.clear(),

            RunEvent::InstallingDependencies { count } => {
                self.start(format!("Installing {count} package(s)..."));
            }

            RunEvent::Applying { name, .. } => {
                self.start(name);
            }

            RunEvent::Finished { .. }
            | RunEvent::DependenciesInstalled { .. }
            | RunEvent::DependenciesResolved { .. } => {
                self.clear();
            }
        }
    }

    fn start(&mut self, message: String) {
        self.clear();
        let bar = ProgressBar::new_spinner();
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        bar.set_message(message);
        self.spinner = Some(bar);
    }

    fn clear(&mut self) {
        if let Some(bar) = self.spinner.take() {
            bar.finish_and_clear();
        }
    }
}

impl Drop for ProgressView {
    fn drop(&mut self) {
        self.clear();
    }
}

#[cfg(test)]
#[path = "progress_test.rs"]
mod progress_test;
