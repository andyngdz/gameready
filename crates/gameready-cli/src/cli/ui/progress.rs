//! Live progress on stderr while steps run.
//!
//! Each sub-phase prints as a completed line when the next one starts, so the
//! user sees a growing checklist rather than a single spinner that replaces
//! itself.

use console::style;
use gameready_core::improvement::OutcomeKind;
use gameready_core::run::RunEvent;
use indicatif::ProgressBar;

/// Renders step progress as a checklist on stderr.
pub struct ProgressView {
    spinner: Option<ProgressBar>,
    last_phase: Option<String>,
}

impl ProgressView {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spinner: None,
            last_phase: None,
        }
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
                self.finish_phase();
                self.start(name);
            }

            RunEvent::StepProgress { message, .. } => {
                self.finish_phase();
                self.last_phase = Some(message.clone());
                self.start(message);
            }

            RunEvent::Finished { kind, .. } => {
                self.finish_phase_with(kind);
                self.clear();
            }

            RunEvent::DependenciesInstalled { .. } | RunEvent::DependenciesResolved { .. } => {
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

    /// Stops the spinner and leaves a completed line on screen.
    fn settle(&mut self, message: String) {
        if let Some(bar) = self.spinner.take() {
            bar.finish_with_message(message);
        }
    }

    /// Prints the previous phase as a completed line.
    fn finish_phase(&mut self) {
        if let Some(phase) = self.last_phase.take() {
            self.settle(format!("  {} {phase}", style("✓").green()));
        }
    }

    /// Prints the last phase with a symbol matching the outcome.
    fn finish_phase_with(&mut self, kind: OutcomeKind) {
        if let Some(phase) = self.last_phase.take() {
            let styled = match kind {
                OutcomeKind::Applied | OutcomeKind::AlreadySet => style("✓").green(),
                OutcomeKind::Failed => style("✗").red(),
                OutcomeKind::Skipped | OutcomeKind::NotApplicable => style("-").dim(),
            };
            self.settle(format!("  {styled} {phase}"));
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
