//! Live progress on stderr while steps run.
//!
//! Each sub-phase prints as a completed line when the next one starts, so the
//! user sees a growing checklist rather than a single spinner that replaces
//! itself.

use gameready_core::run::RunEvent;
use indicatif::ProgressBar;

use crate::cli::ui::layout::Mark;

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

            // Held-open steps belong on the plan screen the user reads a
            // moment later. A line here would land inside the probing spinner,
            // before they have been told what any of it is for.
            RunEvent::Deferred { .. } => {}

            RunEvent::Planned { .. } => self.clear(),

            RunEvent::Reprobing { step, after } => {
                self.recheck(&format!(
                    "Re-checking {step}, now that {after} has finished"
                ));
            }

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
                self.finish_phase_with(Mark::of(kind));
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

    /// Leaves a line on screen for the run looking again, which belongs to no
    /// single step: it is the boundary between the step that just finished and
    /// the one its success brought back.
    fn recheck(&mut self, message: &str) {
        self.finish_phase();
        self.start(message.to_owned());
        self.settle(format!("  {} {message}", Mark::Recheck.glyph()));
    }

    /// Prints the previous phase as a completed line.
    ///
    /// A sub-phase that finished is a change that landed, so it carries the
    /// same mark the summary will print for it a second later.
    fn finish_phase(&mut self) {
        self.finish_phase_with(Mark::Applied);
    }

    /// Prints the last phase with the mark its outcome calls for.
    fn finish_phase_with(&mut self, mark: Mark) {
        if let Some(phase) = self.last_phase.take() {
            self.settle(format!("  {} {phase}", mark.glyph()));
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
