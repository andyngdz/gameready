//! The shapes every screen is built from: titles, rows, labelled paragraphs.

use std::fmt;

use console::style;

use super::marks::Mark;
use super::width::width;

/// How wide the label column in a labelled paragraph is.
///
/// A gutter rather than a share of the line. The labels are a fixed vocabulary
/// ("Why", "Needs", "Here"), so widening this with the terminal would only
/// push the text further from the word that names it.
const LABEL: usize = 10;

/// The indent every line inside a section carries.
const INDENT: usize = 2;

/// The bar down the left of a quoted block.
const QUOTE: &str = "┃";

/// Everything a result row spends on something other than its two ends: the
/// indent, the mark, and a space either side of the leader.
const ROW_FURNITURE: usize = 6;

/// The shortest leader that still reads as one. Below this the dots look like a
/// typo rather than like a line drawn between two columns.
const MIN_LEADER: usize = 4;

/// Writes structured output sections with consistent spacing and separators.
///
/// Every section opens with a title and a blank line, carries indented content,
/// and closes with a separator. Using this for every block keeps the layout in
/// one place, and keeps the terminal width in one place with it.
pub(crate) struct Section<'a, W: fmt::Write> {
    w: &'a mut W,
    width: usize,
}

impl<'a, W: fmt::Write> Section<'a, W> {
    pub(crate) fn new(w: &'a mut W) -> Self {
        Self { w, width: width() }
    }

    /// A section laid out at a width the caller names, for tests that need the
    /// wrapping to be the same on every machine.
    #[cfg(test)]
    pub(crate) fn with_width(w: &'a mut W, width: usize) -> Self {
        Self { w, width }
    }

    /// Title line followed by a blank line.
    pub(crate) fn title(&mut self, text: &str) -> fmt::Result {
        writeln!(self.w, "{text}\n")
    }

    /// A 2-space-indented line with a leading mark and the body text.
    pub(crate) fn marked(&mut self, mark: Mark, text: &str) -> fmt::Result {
        self.flow(&format!("  {} ", mark.glyph()), text)
    }

    /// A 2-space-indented line with no mark.
    pub(crate) fn indented(&mut self, text: &str) -> fmt::Result {
        self.flow("  ", text)
    }

    /// A blank line inside a section.
    pub(crate) fn blank(&mut self) -> fmt::Result {
        writeln!(self.w)
    }

    /// A group heading at the left margin with no blank line after it, for a
    /// screen that stacks several labelled groups (the explain index).
    pub(crate) fn heading(&mut self, text: &str) -> fmt::Result {
        writeln!(self.w, "{text}")
    }

    /// A paragraph wrapped at the left margin, for a closing sentence that runs
    /// past one line, such as the selftest reassurance.
    pub(crate) fn paragraph(&mut self, text: &str) -> fmt::Result {
        self.flow("", text)
    }

    /// A catalog row: a name padded to a shared column, then a dim note. The
    /// eye runs down the column of names rather than following a leader out to a
    /// value, which is what a list read top to bottom wants.
    ///
    /// Written directly rather than through `flow`, whose word-wrapping would
    /// collapse the padding that lines the notes up.
    pub(crate) fn entry(&mut self, name: &str, note: &str, column: usize) -> fmt::Result {
        let padded = format!("{name:<column$}");
        writeln!(self.w, "  {} {}", style(padded).bold(), style(note).dim())
    }

    /// One result row: mark, name, a dotted leader, and the evidence for it at
    /// the right edge.
    ///
    /// The leader is what lets the eye cross a wide terminal from a step's name
    /// to what it actually did. When the two ends leave no room for a leader
    /// worth drawing, the evidence drops to its own indented line instead: a
    /// two-dot leader reads as a mistake, and a wrapped row reads as two rows.
    pub(crate) fn row(&mut self, mark: Mark, name: &str, evidence: &str) -> fmt::Result {
        let ends = console::measure_text_width(name) + console::measure_text_width(evidence);
        let leader = self.width.saturating_sub(ends + ROW_FURNITURE);
        if leader < MIN_LEADER {
            self.marked(mark, name)?;
            return self.sub(&style(evidence).dim().to_string());
        }
        writeln!(
            self.w,
            "  {} {name} {} {}",
            mark.glyph(),
            style(".".repeat(leader)).dim(),
            style(evidence).dim()
        )
    }

    /// A label, then a rule running out to the right edge.
    ///
    /// Heads a question with where it sits in the run. The rule is what makes
    /// it read as a divider rather than as another line of the screen above it,
    /// which matters when four questions arrive one after another.
    pub(crate) fn banner(&mut self, label: &str) -> fmt::Result {
        let used = console::measure_text_width(label);
        let rule = "-".repeat(self.width.saturating_sub(used + 1));
        writeln!(self.w, "{label} {}", style(rule).dim())
    }

    /// A block with a bar down its left edge.
    ///
    /// For words that are about something other than gameready: a package's own
    /// description, quoted while the user decides whether to take it. The bar
    /// carries on down the wrapped lines, so a three-line description reads as
    /// one block rather than as three loose sentences.
    pub(crate) fn quoted(&mut self, text: &str) -> fmt::Result {
        let bar = style(QUOTE).green();
        for line in Self::wrap(text, self.width.saturating_sub(INDENT + 2)) {
            writeln!(self.w, "  {bar} {line}")?;
        }
        Ok(())
    }

    /// A 5-space-indented sub-line under a marked line.
    pub(crate) fn sub(&mut self, text: &str) -> fmt::Result {
        self.flow("     ", text)
    }

    /// A label/value pair nested under a catalog entry: the label dimmed in a
    /// shared column, the value beside it, indented one level past the entry.
    ///
    /// Written directly rather than through `flow`, whose word-wrapping would
    /// collapse the padding that lines the values up across rows.
    pub(crate) fn detail(&mut self, label: &str, value: &str, column: usize) -> fmt::Result {
        let padded = format!("{label:<column$}");
        writeln!(self.w, "    {} {}", style(padded).dim(), value)
    }

    /// Writes `text` after `prefix`, wrapped to the layout width, with every
    /// line after the first indented to the column the text started at.
    ///
    /// The hanging indent is the whole point: a step name that runs onto a
    /// second line has to stay clear of the gutter, or the wrapped remainder
    /// reads as a step of its own.
    fn flow(&mut self, prefix: &str, text: &str) -> fmt::Result {
        let gutter = console::measure_text_width(prefix);
        let hanging = " ".repeat(gutter);
        let mut opening = Some(prefix);

        for line in Self::wrap(text, self.width.saturating_sub(gutter)) {
            let lead = opening.take().unwrap_or(&hanging);
            writeln!(self.w, "{}", format!("{lead}{line}").trim_end())?;
        }
        Ok(())
    }

    /// A row whose short evidence sits inline after a dim separator.
    ///
    /// The value sits after the name as "name · note" and reflows with it,
    /// which reads better for the short values the doctor and rollback screens
    /// show than pushing each one out to the right edge.
    pub(crate) fn noted(&mut self, mark: Mark, name: &str, note: &str) -> fmt::Result {
        let text = format!(
            "{} {}",
            style(name).bold(),
            style(format!("· {note}")).dim()
        );
        self.marked(mark, &text)
    }

    /// A label in its own column, with the text wrapped and aligned under it.
    ///
    /// The label is written once and the following lines are blank in that
    /// column, so a paragraph reads as one answer rather than as a list of
    /// unlabelled fragments.
    pub(crate) fn labelled(&mut self, label: &str, text: &str) -> fmt::Result {
        let mut pending = Some(label);
        for line in Self::wrap(text, self.width - INDENT - LABEL) {
            let shown = pending.take().unwrap_or("");
            writeln!(self.w, "  {shown:<LABEL$}{line}")?;
        }
        Ok(())
    }

    /// Splits text into lines that fit `body` columns, breaking between words.
    ///
    /// Measured in columns rather than in bytes or chars, so a word already
    /// wrapped in colour codes does not push the line over early. A word longer
    /// than the column is left whole rather than cut: an overlong line is ugly,
    /// but a package name or a path broken across two lines cannot be copied.
    fn wrap(text: &str, body: usize) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut current = String::new();

        for word in text.split_whitespace() {
            let grown =
                console::measure_text_width(&current) + 1 + console::measure_text_width(word);
            if !current.is_empty() && grown > body {
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
        writeln!(self.w, "{}", style("-".repeat(self.width)).dim())
    }
}

#[cfg(test)]
#[path = "section_test.rs"]
mod section_test;
