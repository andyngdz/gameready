//! Rendering what a run did.

use std::fmt::Write as _;
use std::path::Path;

use gameready_core::run::RunReport;

/// Renders the per-step lines and the closing summary.
///
/// Returns a string rather than printing, so the whole screen is snapshot
/// testable and `main` owns the one write to stdout.
#[must_use]
pub fn render(report: &RunReport, journal: &Path) -> String {
    let mut out = String::new();
    out.push('\n');

    for step in &report.steps {
        let _ = writeln!(out, "  {} {}", mark(step.outcome.label()), step.name);
        if let Some(detail) = step.outcome.detail() {
            let _ = writeln!(out, "      {detail}");
        }
    }

    let neither = report.steps.len() - report.applied() - report.failed();
    let _ = write!(
        out,
        "\nSummary   {} applied, {neither} not applied, {} failed   {:.1?}\n",
        report.applied(),
        report.failed(),
        report.took,
    );

    let _ = write!(
        out,
        "\n  Undo this run   gameready rollback --run {}\n  Journal         {}\n",
        report.run,
        journal.display(),
    );

    out
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
