//! Asking the user which games to set up.

use anyhow::Result;
use console::style;
use gameready_core::steam::GameSetup;

use crate::cli::ui::theme;

/// The question itself, and the reassurance under it.
const QUESTION: &str = "Which games should I set up?";

/// Why picking nothing is safe, and why picking wrong is not permanent.
const SCOPE: &str = "Only the ones you pick are touched. You can rerun this later.";

/// The keys, in the order a user reaches for them.
/// Every key named the same way: as the word on the key.
///
/// The arrows used to be drawn as glyphs, which put a second right-pointing
/// mark on a screen whose cursor is already one. A reader then has to work out
/// that one of them is a key and the other is where they are.
const KEYS: &str = "space toggle · right all · left none · enter continue · esc skip";

/// What a game gets when a profile of its own matched.
const TUNED: &str = "tuned profile";

/// What a game gets with no profile: the wrapper every game gets.
const PLAIN: &str = "gamemode";

/// One row in the picker.
///
/// Wraps the index because `inquire` hands back the chosen values, and matching
/// them by their rendered text would break the moment two games share a name.
struct Choice {
    index: usize,
    label: String,
}

impl Choice {
    /// How one game reads in the picker.
    ///
    /// Names what the game gets, not whether a file matched: that is
    /// gameready's business, not the user's. Padded to a shared column so the
    /// eye runs down the names and finds what each one gets in the same place
    /// every time.
    fn label(setup: &GameSetup, column: usize) -> String {
        let gets = if setup.has_profile() { TUNED } else { PLAIN };
        format!(
            "{:<column$}  {}",
            setup.game.name,
            style(gets).dim(),
            column = column
        )
    }
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Shows the game picker and returns the chosen setups.
///
/// Nothing is pre-selected: the user opts in to each game explicitly.
pub fn choose_games(setups: &[GameSetup]) -> Result<Vec<GameSetup>> {
    if setups.is_empty() {
        return Ok(Vec::new());
    }

    let column = setups
        .iter()
        .map(|setup| console::measure_text_width(&setup.game.name))
        .max()
        .unwrap_or(0);
    let choices: Vec<Choice> = setups
        .iter()
        .enumerate()
        .map(|(index, setup)| Choice {
            index,
            label: Choice::label(setup, column),
        })
        .collect();

    let picked = theme::Asked::new(QUESTION, SCOPE, KEYS)
        .any_of(choices)
        .prompt_skippable()?
        .unwrap_or_default();

    Ok(picked
        .into_iter()
        .filter_map(|choice| setups.get(choice.index).cloned())
        .collect())
}

#[cfg(test)]
#[path = "prompt_test.rs"]
mod prompt_test;
