//! Asking how the launch options should be applied, and applying them.

use std::fmt;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts::SystemFacts;
use gameready_core::infra::steam::{locate_local_config, write_launch_options};
use gameready_core::journal::Journal;
use inquire::Select;

use crate::cli::ui::{LaunchInstructions, LaunchReport, questions::Answers};

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

impl LaunchChoice {
    /// Carries out the choice and returns the text describing what happened.
    ///
    /// Nothing to set is not an error and not a screen: a machine with no
    /// matching game has nothing to say here.
    pub fn carry_out(
        self,
        runner: &dyn CommandRunner,
        facts: &SystemFacts,
        journal: &mut Journal,
        answers: &Answers,
    ) -> Result<String> {
        if answers.targets.is_empty() {
            return Ok(String::new());
        }
        match self {
            Self::ShowForCopying => Ok(LaunchInstructions::new(&answers.selected).to_string()),
            Self::CloseSteamAndWrite => {
                let config =
                    locate_local_config().context("could not find your Steam user config")?;
                let written =
                    write_launch_options(runner, facts, journal, config, answers.targets.clone())?;
                Ok(LaunchReport::new(&written).to_string())
            }
        }
    }
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
