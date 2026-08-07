//! Rendering what a run did.

use std::fmt;
use std::path::Path;

use console::style;
use gameready_core::run::RunReport;

use crate::cli::ui::colors::{outcome_mark, Section};

/// The full report printed after a run completes.
pub struct Summary<'a> {
    report: &'a RunReport,
    journal: &'a Path,
}

impl<'a> Summary<'a> {
    #[must_use]
    pub const fn new(report: &'a RunReport, journal: &'a Path) -> Self {
        Self { report, journal }
    }

    /// The heading over the step list, which has to match what the run did.
    ///
    /// A run where every step was already correct touched nothing, and this
    /// heading is the line a user reads before deciding whether they now have
    /// something to undo.
    fn heading(&self) -> &'static str {
        if self.recorded() {
            "Config changed:"
        } else {
            "Nothing changed:"
        }
    }

    /// Whether this run wrote anything to the journal.
    ///
    /// Only an applied or failed step appends a record. Every other run leaves
    /// the journal exactly as it found it, so there is nothing to undo and
    /// nothing worth pointing a user at.
    fn recorded(&self) -> bool {
        self.report.applied() + self.report.failed() > 0
    }

    /// The closing line: what the whole run amounts to.
    fn verdict(&self) -> String {
        let failed = self.report.failed();
        if failed > 0 {
            return style(format!("{failed} failed")).red().bold().to_string();
        }

        let applied = self.report.applied();
        if applied > 0 {
            return style(format!("{applied} applied"))
                .green()
                .bold()
                .to_string();
        }

        // A dry run with applicable steps is not a system that was already
        // correct, and saying so would send the user away believing there was
        // nothing to do.
        if self.report.mode.mutates() {
            style("Everything is already set up.").green().to_string()
        } else {
            style("Dry run, nothing was touched. Drop --dry-run to apply.")
                .dim()
                .to_string()
        }
    }

    /// The undo command and the record it reads back.
    fn rollback<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        s.title("Rollback saved:")?;
        s.indented(&format!(
            "{}   gameready rollback --run {}",
            style(crate::cli::ui::UNDO).dim(),
            self.report.run
        ))?;
        s.indented(&format!(
            "{} {}",
            style("History Journal saved ->").dim(),
            self.journal.display()
        ))
    }
}

impl fmt::Display for Summary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);

        s.title(self.heading())?;
        for step in &self.report.steps {
            let mark = outcome_mark(step.outcome.kind());
            let detail = step
                .outcome
                .detail()
                .map(|d| format!(" {}", style(format!("({d})")).dim()))
                .unwrap_or_default();
            s.marked(&mark, &format!("{}{detail}", step.name))?;
        }

        // A run that recorded nothing has no undo command to offer and no new
        // record to point at, so the section goes entirely rather than putting
        // a heading over two lines that contradict it.
        if self.recorded() {
            s.end()?;
            self.rollback(&mut s)?;
        }

        s.blank()?;
        s.indented(&self.verdict())?;
        s.end()
    }
}

#[cfg(test)]
#[path = "summary_test.rs"]
mod summary_test;
