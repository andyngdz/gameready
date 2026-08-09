//! How every screen is laid out: how wide, which marks, which shapes.
//!
//! One owner for all three. A second call site that reaches for its own tick
//! or its own column count is how the live region and the summary end up
//! disagreeing about what the same run just did.

/// The gap between the name column and the evidence beside it.
///
/// Shared by the two renderers that draw a result column, `Section::row` and
/// `ResultTable`, so a screen rendered all at once and the live region printed
/// row by row leave the same space between a name and its value.
const GAP: usize = 4;

mod marks;
mod section;
mod table;
mod width;

pub(crate) use marks::Mark;
pub(crate) use section::Section;
pub(crate) use table::ResultTable;
/// Exposed for the tests that assert a rendered screen fits the terminal it
/// will be printed to. No renderer calls this: they all take the width from
/// `Section`, which is what stops a second answer to the same question.
#[cfg(test)]
pub(crate) use width::width;
