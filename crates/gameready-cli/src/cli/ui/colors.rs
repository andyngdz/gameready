//! Shared formatting for CLI output sections.

use std::fmt;

use console::style;
use gameready_core::improvement::OutcomeKind;

const SEPARATOR: &str = "--------------------------------------";

/// How wide the label column in a labelled paragraph is.
const LABEL: usize = 10;

/// How wide the body of a labelled paragraph is.
///
/// Kept inside 80 columns with the two-space indent and the label column in
/// front of it, because nothing here re-wraps to the real terminal width and a
/// longer line breaks mid-word against the left margin.
const BODY: usize = 66;

/// The gutter mark for a step outcome.
///
/// Applied and already-set carry different marks on purpose. A tick next to a
/// step the run did not touch tells the user their machine changed, and the
/// next thing they do with that belief is roll back something that was never
/// applied.
pub(crate) fn outcome_mark(kind: OutcomeKind) -> String {
    match kind {
        OutcomeKind::Applied => style("\u{2713}").green().to_string(),
        OutcomeKind::AlreadySet => style("=").green().dim().to_string(),
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

    /// A blank line inside a section.
    pub(crate) fn blank(&mut self) -> fmt::Result {
        writeln!(self.w)
    }

    /// A 5-space-indented sub-line under a marked line.
    pub(crate) fn sub(&mut self, text: &str) -> fmt::Result {
        writeln!(self.w, "     {text}")
    }

    /// A label in its own column, with the text wrapped and aligned under it.
    ///
    /// The label is written once and the following lines are blank in that
    /// column, so a paragraph reads as one answer rather than as a list of
    /// unlabelled fragments.
    pub(crate) fn labelled(&mut self, label: &str, text: &str) -> fmt::Result {
        let mut pending = Some(label);
        for line in Self::wrap(text) {
            let shown = pending.take().unwrap_or("");
            writeln!(self.w, "  {shown:<LABEL$}{line}")?;
        }
        Ok(())
    }

    /// Splits text into lines that fit the body column, breaking between words.
    fn wrap(text: &str) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            if !current.is_empty() && current.len() + 1 + word.len() > BODY {
                lines.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }

        lines.push(current);
        lines
    }

    /// Separator that closes the section.
    pub(crate) fn end(&mut self) -> fmt::Result {
        writeln!(self.w, "{}", style(SEPARATOR).dim())
    }
}

#[cfg(test)]
#[path = "colors_test.rs"]
mod colors_test;
