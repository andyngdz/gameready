//! Asking how the launch options should be applied.

use std::fmt;

use anyhow::Result;
use inquire::Select;

/// What the user wants done about Steam's launch options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchChoice {
    /// Quit Steam and write the options into its config.
    ///
    /// Quitting first is not a convenience: Steam holds its config in memory
    /// and rewrites the file when it exits, so a write made while it runs is
    /// thrown away without a word.
    CloseSteamAndWrite,

    /// Print the exact string and leave Steam alone.
    ShowForCopying,
}

impl fmt::Display for LaunchChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CloseSteamAndWrite => "Close Steam and set the launch options for me",
            Self::ShowForCopying => "Just show me the line, I will paste it myself",
        })
    }
}

/// Asks which way to apply the launch options.
///
/// Escaping the prompt means showing the line rather than closing Steam: the
/// reversible, nothing-happens answer is the one an interrupted prompt should
/// land on.
pub fn choose_how_to_apply(games: usize) -> Result<LaunchChoice> {
    let question = format!("Launch options for {games} game(s). How should they be set?");
    let answer = Select::new(
        &question,
        vec![
            LaunchChoice::CloseSteamAndWrite,
            LaunchChoice::ShowForCopying,
        ],
    )
    .with_help_message("Steam has to be closed to set them; it overwrites its config when it quits")
    .prompt_skippable()?;

    Ok(answer.unwrap_or(LaunchChoice::ShowForCopying))
}

#[cfg(test)]
#[path = "launch_choice_test.rs"]
mod launch_choice_test;
