//! Reporting what the launch-options write did.

use std::fmt;

use console::style;
use gameready_core::improvement::OutcomeKind;
use gameready_core::run::RunReport;

use crate::cli::ui::colors::outcome_mark;

/// The outcome of writing Steam's launch options.
pub struct LaunchReport<'a> {
    report: &'a RunReport,
}

impl<'a> LaunchReport<'a> {
    #[must_use]
    pub const fn new(report: &'a RunReport) -> Self {
        Self { report }
    }
}

impl fmt::Display for LaunchReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for step in &self.report.steps {
            let kind = step.outcome.kind();
            match kind {
                OutcomeKind::Applied => {
                    writeln!(
                        f,
                        "  {} Launch options set. Steam is restarting.",
                        outcome_mark(kind)
                    )?;
                }
                OutcomeKind::AlreadySet => {
                    writeln!(
                        f,
                        "  {} {}",
                        outcome_mark(kind),
                        style("Launch options already set.").dim()
                    )?;
                }
                OutcomeKind::Failed => {
                    let detail = step.outcome.detail().unwrap_or_default();
                    writeln!(f, "  {} Launch options: {detail}", outcome_mark(kind))?;
                }
                OutcomeKind::Skipped | OutcomeKind::NotApplicable => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "launch_report_test.rs"]
mod launch_report_test;
