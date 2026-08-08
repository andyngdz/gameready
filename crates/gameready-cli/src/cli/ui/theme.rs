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

/// The pointer at the row the keyboard is on.
const CURSOR: &str = "  ▸";

/// What sits where the pointer would be on every other row, so the options do
/// not shift sideways as the cursor moves.
const NO_CURSOR: &str = "   ";

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

/// The styling every prompt in a run shares.
///
/// One place for it, so the questions look like one conversation rather than
/// like four prompts from four libraries. Set on each prompt rather than
/// globally: `inquire`'s global config is a process-wide mutable, and a run
/// that sets it once from `init` would be quietly styling `apply` too.
#[must_use]
pub fn questions<'a>() -> RenderConfig<'a> {
    let mut config = RenderConfig::default()
        // No prefix on the question: it already reads as one, and the header
        // above it has said where in the run it sits.
        .with_prompt_prefix(Styled::new(""))
        .with_answered_prompt_prefix(Styled::new("✓").with_fg(Color::LightGreen))
        .with_highlighted_option_prefix(Styled::new(CURSOR).with_fg(Color::LightBlue))
        .with_selected_checkbox(Styled::new(CHECKED).with_fg(Color::LightGreen))
        .with_unselected_checkbox(Styled::new(UNCHECKED).with_fg(Color::DarkGrey))
        .with_selected_option(Some(StyleSheet::new().with_attr(Attributes::BOLD)))
        .with_help_message(StyleSheet::new().with_fg(Color::DarkGrey))
        .with_canceled_prompt_indicator(Styled::new("skipped").with_fg(Color::DarkGrey));

    // Not exposed as a builder, and it has to match the cursor's width or the
    // rows jump left and right as the cursor moves down them.
    config.unhighlighted_option_prefix = Styled::new(NO_CURSOR);
    config
}

#[cfg(test)]
#[path = "theme_test.rs"]
mod theme_test;
