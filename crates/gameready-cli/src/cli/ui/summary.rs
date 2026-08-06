//! Rendering what a run did.

use std::fmt;
use std::path::Path;

use console::style;
use gameready_core::run::RunReport;

use crate::cli::ui::colors::{Section, outcome_mark};

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
}

impl fmt::Display for Summary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);

        s.title("Config changed:")?;
        for step in &self.report.steps {
            let mark = outcome_mark(step.outcome.kind());
            let detail = step
                .outcome
                .detail()
                .map(|d| format!(" {}", style(format!("({d})")).dim()))
                .unwrap_or_default();
            s.marked(&mark, &format!("{}{detail}", step.name))?;
        }
        s.end()?;

        let applied = self.report.applied();
        let failed = self.report.failed();

        s.title("Rollback saved:")?;
        if applied > 0 || failed > 0 {
            s.indented(&format!(
                "{}   gameready rollback --run {}",
                style("Undo").dim(),
                self.report.run
            ))?;
        }
        s.indented(&format!(
            "{} {}",
            style("History Journal saved ->").dim(),
            self.journal.display()
        ))?;

        if failed > 0 {
            s.indented(&style(format!("{failed} failed")).red().bold().to_string())?;
        } else if applied > 0 {
            s.indented(
                &style(format!("{applied} applied"))
                    .green()
                    .bold()
                    .to_string(),
            )?;
        } else {
            s.indented(&style("Everything is already set up.").green().to_string())?;
        }
        s.end()
    }
}

#[cfg(test)]
#[path = "summary_test.rs"]
mod summary_test;
