//! Shared formatting for CLI output sections.

use std::fmt;

use console::style;
use gameready_core::improvement::OutcomeKind;

const SEPARATOR: &str = "--------------------------------------";

/// The gutter mark for a step outcome.
pub(crate) fn outcome_mark(kind: OutcomeKind) -> String {
    match kind {
        OutcomeKind::Applied | OutcomeKind::AlreadySet => style("\u{2713}").green().to_string(),
        OutcomeKind::Failed => style("\u{2718}").red().bold().to_string(),
        OutcomeKind::Skipped | OutcomeKind::NotApplicable => style("~").dim().to_string(),
    }
}

/// Writes structured output sections with consistent spacing and separators.
///
/// Every section opens with a title and a blank line, carries indented content,
/// and closes with a separator. Using this for every block keeps the layout in
/// one place.
pub(crate) struct Section<'a, W: fmt::Write> {
    w: &'a mut W,
}

impl<'a, W: fmt::Write> Section<'a, W> {
    pub(crate) fn new(w: &'a mut W) -> Self {
        Self { w }
    }

    /// Title line followed by a blank line.
    pub(crate) fn title(&mut self, text: &str) -> fmt::Result {
        writeln!(self.w, "{text}\n")
    }

    /// A 2-space-indented line with a leading mark and the body text.
    pub(crate) fn marked(&mut self, mark: &str, text: &str) -> fmt::Result {
        writeln!(self.w, "  {mark} {text}")
    }

    /// A 2-space-indented line with no mark.
    pub(crate) fn indented(&mut self, text: &str) -> fmt::Result {
        writeln!(self.w, "  {text}")
    }

    /// A 5-space-indented sub-line under a marked line.
    pub(crate) fn sub(&mut self, text: &str) -> fmt::Result {
        writeln!(self.w, "     {text}")
    }

    /// Separator that closes the section.
    pub(crate) fn end(&mut self) -> fmt::Result {
        writeln!(self.w, "{}", style(SEPARATOR).dim())
    }
}

#[cfg(test)]
#[path = "colors_test.rs"]
mod colors_test;
