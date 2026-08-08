//! Live progress on stderr while a run works.
//!
//! One settled line per step, carrying the evidence for what it did, and one
//! spinner at the bottom for the step still running. A step that takes a while
//! reports its sub-phases into that spinner rather than leaving a line each, so
//! the region reads as a checklist of the run rather than as a log of it.

use std::collections::HashMap;

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::region::LiveRegion;
use crate::cli::ui::{name_column, short_names, tunings};
use gameready_core::improvement::ImprovementId;
use gameready_core::run::{Mode, RunEvent};

/// Renders step progress as a checklist on stderr.
pub struct ProgressView {
    region: LiveRegion,
    /// The step being applied, kept so its sub-phases can be shown against it.
    applying: Option<String>,
    names: HashMap<ImprovementId, String>,
}

impl ProgressView {
    /// A view for the planning phase.
    ///
    /// Carries the name catalog like the sweep does. Planning is not only a
    /// count: every step the probe settles leaves a row here, and a view with
    /// no catalog prints each one by the sentence the event carried rather than
    /// by the name the rest of the run calls it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            region: LiveRegion::default(),
            applying: None,
            names: short_names(),
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
        Self::new()
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

            RunEvent::StepBytes { done, total, .. } => self.landed(done, total),

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
        self.region.say(counted);
    }

    /// What the running step is busy with, shown against its own name.
    fn sub_phase(&mut self, message: &str) {
        let line = match &self.applying {
            Some(name) => format!("{name} · {message}"),
            None => message.to_owned(),
        };
        self.region.say(line);
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
        self.region.spin(message);
    }

    fn clear(&mut self) {
        self.region.clear();
    }

    /// How much of a download has landed, against the name of the step doing it.
    fn landed(&mut self, done: u64, total: u64) {
        let name = self.applying.clone().unwrap_or_default();
        self.region.count(&name, done, total);
    }

    /// Takes the live line down and prints the finished one in its place.
    fn settle(&mut self, message: &str) {
        self.region.settle();
        print(&format!("{message}\n"));
    }

    /// One finished step, with the evidence for what it did.
    fn settle_row(&mut self, mark: Mark, name: &str, detail: Option<&str>) {
        let line = match detail {
            Some(evidence) => result_row(mark, name, evidence, name_column(&self.names)),
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

/// A result row, with its evidence in the shared column.
fn result_row(mark: Mark, name: &str, evidence: &str, column: usize) -> String {
    let mut out = String::new();
    let mut section = Section::new(&mut out);
    let _ = section.row(mark, name, evidence, column);
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
