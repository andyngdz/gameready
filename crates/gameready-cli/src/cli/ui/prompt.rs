//! Asking the user which games to set up.

use anyhow::Result;
use gameready_core::steam::GameSetup;
use inquire::MultiSelect;

/// One row in the picker.
///
/// Wraps the index because `inquire` hands back the chosen values, and matching
/// them by their rendered text would break the moment two games share a name.
struct Choice {
    index: usize,
    label: String,
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

    let choices: Vec<Choice> = setups
        .iter()
        .enumerate()
        .map(|(index, setup)| Choice {
            index,
            label: label(setup),
        })
        .collect();

    let picked = MultiSelect::new("Which games should gameready set up?", choices)
        .with_help_message("space toggles, enter confirms, esc picks none")
        .prompt_skippable()?
        .unwrap_or_default();

    Ok(picked
        .into_iter()
        .filter_map(|choice| setups.get(choice.index).cloned())
        .collect())
}

/// How one game reads in the picker.
pub(super) fn label(setup: &GameSetup) -> String {
    if setup.has_profile() {
        format!("{}  (has a profile)", setup.game.name)
    } else {
        format!("{}  (core tuning only)", setup.game.name)
    }
}

#[cfg(test)]
#[path = "prompt_test.rs"]
mod prompt_test;
