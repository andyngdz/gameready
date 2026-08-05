//! Rendering what a rollback did.

use std::fmt;
use std::path::Path;

use gameready_core::rollback::RollbackReport;

/// A rollback report paired with where its journal lives, ready to print.
///
/// See [`crate::cli::ui::Summary`] for why this is a `Display` view rather
/// than a function returning `String`.
pub struct RollbackSummary<'a> {
    report: &'a RollbackReport,
    journal: &'a Path,
}

impl<'a> RollbackSummary<'a> {
    /// Pairs a report with the journal path shown at the bottom.
    #[must_use]
    pub const fn new(report: &'a RollbackReport, journal: &'a Path) -> Self {
        Self { report, journal }
    }
}

impl fmt::Display for RollbackSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "Rollback of run {}", self.report.run)?;

        for undo in &self.report.undos {
            let mark = if undo.outcome.is_failure() {
                "!!"
            } else {
                "ok"
            };
            writeln!(f, "  {mark} {}", undo.outcome.describe())?;
        }

        writeln!(f)?;
        writeln!(
            f,
            "Summary   {} reverted, {} failed",
            self.report.reverted(),
            self.report.failed(),
        )?;

        writeln!(f)?;
        writeln!(f, "  Journal   {}", self.journal.display())
    }
}

#[cfg(test)]
#[path = "rollback_test.rs"]
mod rollback_test;
