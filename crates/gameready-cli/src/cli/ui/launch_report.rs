//! Reporting what the launch-options write did.

use std::fmt;

use gameready_core::run::RunReport;

/// The outcome of writing Steam's launch options.
///
/// Rendered on its own rather than folded into the main summary because it is a
/// separate run of a separate step, and because the user has just had their
/// Steam closed and should see plainly what that bought them.
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
        writeln!(f, "\nLaunch options")?;
        for step in &self.report.steps {
            writeln!(f, "  {}", step.outcome.label())?;
            if let Some(detail) = step.outcome.detail() {
                writeln!(f, "    {detail}")?;
            }
        }
        writeln!(
            f,
            "  Steam was closed to write these; start it again when ready."
        )
    }
}

#[cfg(test)]
#[path = "launch_report_test.rs"]
mod launch_report_test;
