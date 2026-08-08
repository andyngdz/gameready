//! Rendering a selftest run.

use std::fmt;

use console::style;
use gameready_core::run::{RevertCheck, SelftestResult, StepSelftest};

use crate::cli::ui::layout::{Mark, ResultTable, Section};
use crate::cli::ui::{name_column, short_names};

/// The reassurance under the counts: whatever the test found, it put the
/// machine back, so a failed test never leaves a half-applied tuning behind.
const REASSURANCE: &str =
    "Your machine is as it was: selftest undoes its own work even when the test itself fails.";

/// The selftest results as printable lines.
///
/// Each row leads with its mark rather than with the step id, so a failure is
/// visible while the list scrolls past.
pub struct SelftestSummary<'a> {
    results: &'a [StepSelftest],
}

impl<'a> SelftestSummary<'a> {
    #[must_use]
    pub const fn new(results: &'a [StepSelftest]) -> Self {
        Self { results }
    }

    /// The verdict, with the reassurance running on from it.
    ///
    /// One paragraph rather than two lines: the reassurance is only worth
    /// reading because of the verdict in front of it, and a line break between
    /// them turns it into a standing disclaimer nobody reads twice.
    fn summary<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        let total = self.results.len();
        let failed = self
            .results
            .iter()
            .filter(|result| result.is_failure())
            .count();
        let headline = if failed == 0 {
            style(format!("All {total} passed.")).green()
        } else {
            style(format!("{failed} of {total} failed.")).red()
        };
        s.paragraph(&format!("{} {}", headline.bold(), style(REASSURANCE).dim()))
    }

    /// The gutter mark for how one step's cycle ended.
    fn mark(result: &SelftestResult) -> Mark {
        match result {
            SelftestResult::Passed { .. } => Mark::Applied,
            SelftestResult::Skipped { .. } => Mark::Skipped,
            SelftestResult::ProbeFailed { .. } | SelftestResult::Failed { .. } => Mark::Failed,
        }
    }

    /// What the step's cycle reads as after the subject.
    fn note(result: &SelftestResult) -> String {
        match result {
            SelftestResult::Passed { reverted } => match reverted {
                RevertCheck::Confirmed => "applied, verified, reverted".to_owned(),
                RevertCheck::NotApplicable => "applied, verified, nothing to revert".to_owned(),
            },
            SelftestResult::Skipped { reason } => format!("skipped, {reason}"),
            SelftestResult::ProbeFailed { error } => format!("probe failed: {error}"),
            SelftestResult::Failed { phase, detail } => {
                format!("{} failed: {detail}", phase.label())
            }
        }
    }
}

impl fmt::Display for SelftestSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = short_names();
        let column = name_column(&names);
        let mut s = Section::new(f);
        s.blank()?;
        s.title("Applying each tuning, then undoing it, to prove both halves work")?;

        let mut table = ResultTable::new(column);
        for result in self.results {
            let subject = names
                .get(&result.step)
                .cloned()
                .unwrap_or_else(|| result.step.to_string());
            table.row(
                Self::mark(&result.result),
                &subject,
                &Self::note(&result.result),
            );
        }
        s.heading(&table.to_string())?;

        s.blank()?;
        self.summary(&mut s)
    }
}

#[cfg(test)]
#[path = "selftest_test.rs"]
mod selftest_test;
