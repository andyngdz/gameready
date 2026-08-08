//! How wide to lay out, and why it is not simply the terminal's own answer.

use terminal_size::{terminal_size, Width};

/// The layout width when there is no terminal to ask, such as a pipe.
const PIPED: usize = 80;

/// The narrowest layout, below which the label column and the body collide.
const NARROWEST: usize = 60;

/// The widest layout. Past this a line of prose is too long to track back to
/// its own start, which is why books are not printed on a metre-wide page.
const WIDEST: usize = 100;

/// How wide to lay out, in columns.
///
/// Clamped rather than taken raw, between `NARROWEST` and `WIDEST`. `COLUMNS`
/// wins over the real terminal so a test can pin the layout.
pub(crate) fn width() -> usize {
    usable(asked_width())
}

/// What this run was told the terminal is, before the clamp.
fn asked_width() -> usize {
    columns_env()
        .or_else(|| terminal_size().map(|(Width(columns), _)| usize::from(columns)))
        .unwrap_or(PIPED)
}

/// Brings a requested column count into the range this lays out at.
fn usable(asked: usize) -> usize {
    asked.clamp(NARROWEST, WIDEST)
}

/// The `COLUMNS` override, when it is set to something usable.
fn columns_env() -> Option<usize> {
    std::env::var("COLUMNS").ok()?.trim().parse().ok()
}

#[cfg(test)]
#[path = "width_test.rs"]
mod width_test;
