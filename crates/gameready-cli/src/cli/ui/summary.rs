//! Rendering what a run did.

use std::fmt;
use std::path::Path;

use gameready_core::run::RunReport;

/// A run report paired with where its journal lives, ready to print.
///
/// A view struct rather than a function returning `String`. `Display` gives the
/// `?` operator somewhere to send a formatting error, so no line has to discard
/// a `Result`, and `println!("{}", ...)` writes straight to stdout without
/// building the whole screen in memory first.
pub struct Summary<'a> {
    report: &'a RunReport,
    journal: &'a Path,
}

impl<'a> Summary<'a> {
    /// Pairs a report with the journal path shown at the bottom.
    #[must_use]
    pub const fn new(report: &'a RunReport, journal: &'a Path) -> Self {
        Self { report, journal }
    }
}

impl fmt::Display for Summary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;

        for step in &self.report.steps {
            writeln!(f, "  {} {}", mark(step.outcome.label()), step.name)?;
            if let Some(detail) = step.outcome.detail() {
                writeln!(f, "      {detail}")?;
            }
        }

        let neither = self.report.steps.len() - self.report.applied() - self.report.failed();
        writeln!(f)?;
        writeln!(
            f,
            "Summary   {} applied, {neither} not applied, {} failed   {:.1?}",
            self.report.applied(),
            self.report.failed(),
            self.report.took,
        )?;

        writeln!(f)?;
        writeln!(
            f,
            "  Undo this run   gameready rollback --run {}",
            self.report.run
        )?;
        writeln!(f, "  Journal         {}", self.journal.display())
    }
}

/// The two-character gutter for an outcome label.
const fn mark(label: &str) -> &'static str {
    match label.as_bytes() {
        b"applied" => "ok",
        b"failed" => "!!",
        _ => "--",
    }
}

#[cfg(test)]
#[path = "summary_test.rs"]
mod summary_test;
