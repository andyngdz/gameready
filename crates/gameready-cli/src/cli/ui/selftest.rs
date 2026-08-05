//! Rendering a selftest run.

use std::fmt;

use gameready_core::run::{SelftestResult, StepSelftest};

/// The selftest results as printable lines.
///
/// Each line leads with its marker rather than with the step id, so a failure
/// is visible while the list scrolls past.
pub struct SelftestSummary<'a> {
    results: &'a [StepSelftest],
}

impl<'a> SelftestSummary<'a> {
    #[must_use]
    pub const fn new(results: &'a [StepSelftest]) -> Self {
        Self { results }
    }
}

impl fmt::Display for SelftestSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\nSelftest")?;
        for result in self.results {
            let step = &result.step;
            match &result.result {
                SelftestResult::Passed { reverted } => {
                    writeln!(f, "  ok  {step}  {}", reverted.label())?;
                }
                SelftestResult::Skipped { reason } => {
                    writeln!(f, "  --  {step}  skipped, {reason}")?;
                }
                SelftestResult::ProbeFailed { error } => {
                    writeln!(f, "  !!  {step}  probe failed: {error}")?;
                }
                SelftestResult::Failed { phase, detail } => {
                    writeln!(f, "  !!  {step}  {} failed: {detail}", phase.label())?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "selftest_test.rs"]
mod selftest_test;
