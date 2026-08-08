//! How every question is drawn.

use console::style;
use inquire::ui::{Attributes, Color, RenderConfig, StyleSheet, Styled};

use crate::cli::ui::layout::Section;

/// The tick beside a chosen game. The same glyph the summary marks an applied
/// step with, because it means the same thing: this one is in.
const CHECKED: &str = "[✓]";

/// The empty box beside a game that is not in.
// Rule Ignore: RUST039
// Reason: An empty checkbox glyph, not a shell test expression. The scanner
// reads the brackets as `test`; there is no shell anywhere in this file.
const UNCHECKED: &str = "[ ]";

/// The gutter every option sits in.
///
/// The same on the row the keyboard is on as on every other row. Where the
/// reader is is carried by the row itself going bold and green; a glyph in
/// front of it would be a second mark for a fact one already covers, on a
/// screen that has a checkbox to read as well.
///
/// One space rather than the layout's two, because `inquire` writes its own
/// space between the prefix and the box. Two would put the box a column right
/// of the dial the one-of screens draw at the same point.
const GUTTER: &str = " ";

/// The answer a one-of list would take.
///
/// A dial rather than a pointer: on a list where exactly one row wins, where
/// the cursor is and what is chosen are the same fact, and drawing them as two
/// marks asks the reader to work out that they are one.
const PICKED: &str = "  (●)";

/// Every other answer on a one-of list.
const NOT_PICKED: &str = "  ( )";

/// A question, with what it is really asking under it.
///
/// Wrapped here rather than left to `inquire`, which breaks a long line at
/// whatever column the terminal ends at, mid-word. Every other screen in the
/// run wraps between words, and a question is the last place to start splitting
/// words in half.
#[must_use]
pub fn asked(question: &str, detail: &str) -> String {
    let mut message = String::new();
    let mut section = Section::new(&mut message);
    // Writing into a String cannot fail.
    let _ = section
        .heading(question)
        .and_then(|()| section.paragraph(&style(detail).dim().to_string()));
    message
}

/// How each shape of question is drawn.
pub struct Prompts;

impl Prompts {
    /// A list where the answer is one row.
    ///
    /// The row the keyboard is on is the row that would be taken, so it is
    /// drawn as the answer rather than as a pointer at it. This is the only
    /// shape `inquire` gives a one-of list: a `Select` has no checkbox, but it
    /// does have a prefix for the current row and a prefix for the rest, and
    /// using both is a dial.
    #[must_use]
    pub fn choices<'a>() -> RenderConfig<'a> {
        let mut config = Self::shared()
            .with_highlighted_option_prefix(Styled::new(PICKED).with_fg(Color::LightGreen));
        config.unhighlighted_option_prefix = Styled::new(NOT_PICKED).with_fg(Color::DarkGrey);
        config
    }

    /// A list where any number of rows can be on.
    ///
    /// The box says whether a row is on. Where the keyboard is is left to the
    /// row's own weight and colour, so the only mark on the line is the one
    /// that carries a fact the reader has to act on.
    #[must_use]
    pub fn many<'a>() -> RenderConfig<'a> {
        let mut config = Self::shared()
            .with_highlighted_option_prefix(Styled::new(GUTTER))
            .with_selected_checkbox(Styled::new(CHECKED).with_fg(Color::LightGreen))
            .with_unselected_checkbox(Styled::new(UNCHECKED).with_fg(Color::DarkGrey));
        config.unhighlighted_option_prefix = Styled::new(GUTTER);
        config
    }

    /// What every prompt in a run shares.
    ///
    /// One place for it, so the questions look like one conversation rather
    /// than like four prompts from four libraries. Set on each prompt rather
    /// than globally: `inquire`'s global config is a process-wide mutable, and
    /// a run that sets it once from `init` would be quietly styling `apply`
    /// too.
    fn shared<'a>() -> RenderConfig<'a> {
        RenderConfig::default()
            // No prefix on the question: it already reads as one, and the
            // header above it has said where in the run it sits.
            .with_prompt_prefix(Styled::new(""))
            .with_answered_prompt_prefix(Styled::new("✓").with_fg(Color::LightGreen))
            // Bold as well as coloured: weight survives a terminal with no
            // colour, and it is the only thing saying where the keyboard is.
            .with_selected_option(Some(
                StyleSheet::new()
                    .with_attr(Attributes::BOLD)
                    .with_fg(Color::LightGreen),
            ))
            .with_help_message(StyleSheet::new().with_fg(Color::DarkGrey))
            .with_canceled_prompt_indicator(Styled::new("skipped").with_fg(Color::DarkGrey))
    }
}

#[cfg(test)]
#[path = "theme_test.rs"]
mod theme_test;
