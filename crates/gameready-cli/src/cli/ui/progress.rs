//! Live progress on stderr while a run works.
//!
//! One settled line per step, carrying the evidence for what it did, and one
//! spinner at the bottom for the step still running. A step that takes a while
//! reports its sub-phases into that spinner rather than leaving a line each, so
//! the region reads as a checklist of the run rather than as a log of it.

use std::collections::HashMap;

use gameready_core::improvement::ImprovementId;
use gameready_core::run::{Mode, RunEvent};
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::{short_names, tunings};

/// How the spinner line is laid out: indented to the column the settled lines
/// land in, so the line being waited on does not sit left of the ones it turns
/// into a moment later.
const SPINNER: &str = "  {spinner:.blue} {msg}";

/// Renders step progress as a checklist on stderr.
pub struct ProgressView {
    spinner: Option<ProgressBar>,
    /// The step being applied, kept so its sub-phases can be shown against it.
    applying: Option<String>,
    names: HashMap<ImprovementId, String>,
}

impl ProgressView {
    /// A view for the planning phase, which only ever shows a count.
    #[must_use]
    pub fn new() -> Self {
        Self {
            spinner: None,
            applying: None,
            names: HashMap::new(),
        }
    }

    /// A view for the sweep, headed with how much there is to get through.
    ///
    /// The header is printed once rather than kept up to date: the settled
    /// lines scroll up past it, so a running count in the header would be
    /// describing a screen the reader can no longer see. A dry run gets no
    /// header at all, because it applies none of them.
    #[must_use]
    pub fn sweeping(mode: Mode, total: usize) -> Self {
        if mode.mutates() && total > 0 {
            print(&format!("\nApplying {total} {}\n", tunings(total)));
        }
        Self {
            spinner: None,
            applying: None,
            names: short_names(),
        }
    }

    /// Handles one event from the executor.
    pub fn on_event(&mut self, event: RunEvent) {
        match event {
            RunEvent::Probing { done, total, .. } => self.counting(done, total),

            // Held-open steps belong on the plan screen the user reads a
            // moment later. A line here would land inside the probing spinner,
            // before they have been told what any of it is for.
            RunEvent::Deferred { .. } => {}

            RunEvent::Planned { .. } => self.clear(),

            RunEvent::Reprobing { step, after } => self.recheck(&step, &after),

            RunEvent::InstallingDependencies { count } => {
                self.start(format!("Installing {count} package(s)"));
            }

            RunEvent::Applying { step, name } => {
                let shown = self.named(&step, &name);
                self.applying = Some(shown.clone());
                self.start(shown);
            }

            RunEvent::StepProgress { message, .. } => self.sub_phase(&message),

            RunEvent::Finished {
                step,
                name,
                kind,
                detail,
            } => {
                let shown = self.named(&step, &name);
                self.settle_row(Mark::of(kind), &shown, detail.as_deref());
                self.applying = None;
            }

            RunEvent::DependenciesInstalled { .. } | RunEvent::DependenciesResolved { .. } => {
                self.clear();
            }
        }
    }

    /// The name a reader knows this step by, falling back to whatever the event
    /// carried for a step that is not in the catalog.
    fn named(&self, step: &ImprovementId, name: &str) -> String {
        self.names
            .get(step)
            .cloned()
            .unwrap_or_else(|| name.to_owned())
    }

    /// How far the probing sweep has got, updated in place.
    ///
    /// Every probe is the same activity, and a spinner restarted per step would
    /// read as nine separate waits.
    fn counting(&mut self, done: usize, total: usize) {
        let counted = format!("Checking {total} {} · {done} of {total}", tunings(total));
        if let Some(bar) = &self.spinner {
            bar.set_message(counted);
        } else {
            self.start(counted);
        }
    }

    /// What the running step is busy with, shown against its own name.
    fn sub_phase(&mut self, message: &str) {
        let line = match &self.applying {
            Some(name) => format!("{name} · {message}"),
            None => message.to_owned(),
        };
        if let Some(bar) = &self.spinner {
            bar.set_message(line);
        } else {
            self.start(line);
        }
    }

    /// Leaves a line on screen for the run looking again.
    ///
    /// It belongs to no single step: it is the boundary between the step that
    /// just finished and the one its success brought back.
    fn recheck(&mut self, step: &ImprovementId, after: &ImprovementId) {
        let step = self.named(step, &step.to_string());
        let after = self.named(after, &after.to_string());
        let line = marked_line(
            Mark::Recheck,
            &format!("{after} is here now, so {step} can run in this pass"),
        );
        self.settle(&line);
    }

    fn start(&mut self, message: String) {
        self.clear();
        let bar = ProgressBar::new_spinner();
        if let Ok(style) = ProgressStyle::with_template(SPINNER) {
            bar.set_style(style);
        }
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        bar.set_message(message);
        self.spinner = Some(bar);
    }

    fn clear(&mut self) {
        if let Some(bar) = self.spinner.take() {
            bar.finish_and_clear();
        }
    }

    /// Stops the spinner and leaves the finished lines on screen.
    ///
    /// A row whose evidence would not fit beside its name comes back as two
    /// lines, and `finish_with_message` pads whatever it is given to the width
    /// of one bar. So anything with a newline in it is cleared and printed
    /// rather than handed to the bar.
    fn settle(&mut self, message: &str) {
        if let Some(bar) = self.spinner.take() {
            if !message.contains('\n') {
                bar.finish_with_message(message.to_owned());
                return;
            }
            bar.finish_and_clear();
        }
        print(&format!("{message}\n"));
    }

    /// One finished step, with the evidence for what it did.
    fn settle_row(&mut self, mark: Mark, name: &str, detail: Option<&str>) {
        let line = match detail {
            Some(evidence) => result_row(mark, name, evidence),
            None => marked_line(mark, name),
        };
        self.settle(&line);
    }
}

/// A marked line, laid out the way every other screen lays one out.
fn marked_line(mark: Mark, text: &str) -> String {
    let mut out = String::new();
    let mut section = Section::new(&mut out);
    // Writing into a String cannot fail.
    let _ = section.marked(mark, text);
    out.trim_end().to_owned()
}

/// A result row, with the leader drawn between its two ends.
fn result_row(mark: Mark, name: &str, evidence: &str) -> String {
    let mut out = String::new();
    let mut section = Section::new(&mut out);
    let _ = section.row(mark, name, evidence);
    out.trim_end().to_owned()
}

/// Writes straight to stderr, for the lines that are not a spinner.
///
/// Held to the same rule the spinner holds itself to: a redirected run gets the
/// report on stdout and nothing else, so a script parsing the output never has
/// to strip a live region that was never live.
fn print(text: &str) {
    if console::user_attended_stderr() {
        eprint!("{text}");
    }
}

impl Default for ProgressView {
    fn default() -> Self {
        Self::new()
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
