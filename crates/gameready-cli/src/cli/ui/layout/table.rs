//! A list of results, laid out as a table.

use std::fmt;

use comfy_table::presets::NOTHING;
use comfy_table::{Cell, ColumnConstraint, ContentArrangement, Table, Width};
use console::style;

use super::marks::Mark;
use super::width::width;

/// The indent every table carries, matching the rest of the layout.
const INDENT: &str = "  ";

/// Space after a cell, so the columns do not touch.
const PADDING: (u16, u16) = (0, 1);

/// A table of results: a mark, a name, and what the step did.
///
/// The name column is pinned rather than sized to its content. Every screen
/// that lists steps pins it to the same width, so the live region, which prints
/// its rows one at a time and cannot measure a table it has not finished, lines
/// up with the screens that render all at once.
pub(crate) struct ResultTable {
    column: usize,
    rows: Vec<[String; 3]>,
}

impl ResultTable {
    /// An empty table with the name column `column` wide.
    pub(crate) const fn new(column: usize) -> Self {
        Self {
            column,
            rows: Vec::new(),
        }
    }

    /// One result. Long evidence wraps inside its own column.
    pub(crate) fn row(&mut self, mark: Mark, name: &str, evidence: &str) {
        self.rows.push([
            mark.glyph(),
            name.to_owned(),
            style(evidence).dim().to_string(),
        ]);
    }

    /// The rows, laid out.
    ///
    /// Built here rather than as rows arrive because comfy-table grows its
    /// columns from the rows it holds: a constraint set on a column that does
    /// not exist yet is dropped without a word, and the table then sizes itself
    /// to whatever this particular screen happens to list.
    fn built(&self) -> Table {
        let mut table = Table::new();
        table
            .load_style(NOTHING)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(u16::try_from(width().saturating_sub(INDENT.len())).unwrap_or(u16::MAX));

        for row in &self.rows {
            table.add_row(row.iter().map(Cell::new).collect::<Vec<_>>());
        }

        for index in 0..3 {
            if let Some(cell) = table.column_mut(index) {
                cell.set_padding(PADDING);
            }
        }
        if let Some(names) = table.column_mut(1) {
            let pinned = u16::try_from(self.column).unwrap_or(u16::MAX);
            names.set_constraint(ColumnConstraint::Absolute(Width::Fixed(
                pinned.saturating_add(PADDING.1),
            )));
        }
        table
    }
}

impl fmt::Display for ResultTable {
    /// The table, indented to the column the rest of the layout starts at.
    ///
    /// comfy-table lays out from zero and knows nothing about the indent every
    /// other block here carries, so the indent is added per line afterwards
    /// rather than smuggled in as a column of spaces.
    ///
    /// No trailing newline: the caller writes this as one block of a section,
    /// and a block that ends in one would leave a blank line behind it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rendered = self.built().to_string();
        let mut lines = rendered.lines().peekable();
        while let Some(line) = lines.next() {
            write!(f, "{INDENT}{}", line.trim_end())?;
            if lines.peek().is_some() {
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "table_test.rs"]
mod table_test;
