//! Asking how the launch options should be applied, and applying them.

use std::fmt;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts::SystemFacts;
use gameready_core::infra::steam::{
    configs_under, locate_steam_dir, write_steam_settings, SteamSettings,
};
use gameready_core::journal::Journal;
use inquire::Select;

use crate::cli::ui::{questions::Answers, LaunchInstructions, LaunchReport};

/// What the user wants done about the per-game settings Steam holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchChoice {
    /// Quit Steam and write them into its config.
    ///
    /// Quitting first is not a convenience: Steam holds its config in memory
    /// and rewrites the files when it exits, so a write made while it runs is
    /// thrown away without a word.
    CloseSteamAndWrite,

    /// Print what to set and leave Steam alone.
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
        if answers.targets.is_empty() && answers.proton.is_empty() {
            return Ok(String::new());
        }
        match self {
            Self::ShowForCopying => Ok(LaunchInstructions::new(answers).to_string()),
            Self::CloseSteamAndWrite => {
                let steam = locate_steam_dir().context(NO_STEAM)?;
                let configs = configs_under(&steam).context(NO_STEAM)?;
                let settings = SteamSettings {
                    launch: answers.targets.clone(),
                    proton: answers.proton.clone(),
                };
                let written = write_steam_settings(runner, facts, journal, configs, settings)?;
                Ok(LaunchReport::new(&written).to_string())
            }
        }
    }
}

/// What to say when Steam's own files cannot be found.
const NO_STEAM: &str = "could not find your Steam config";

impl fmt::Display for LaunchChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CloseSteamAndWrite => "Close Steam and set them for me",
            Self::ShowForCopying => "Just show me what to set, I will do it myself",
        })
    }
}

/// What a run has to set inside Steam, for the question that asks about it.
pub struct SteamWork {
    /// How many games get launch options.
    pub launch: usize,
    /// How many games get pinned to a Proton version.
    pub proton: usize,
}

impl SteamWork {
    /// The question, naming each thing that will change.
    ///
    /// Both counts are said out loud rather than rolled into one number: the
    /// Proton version is the setting a user is most likely to have chosen
    /// themselves, and it should not arrive as a surprise inside "settings".
    fn question(&self) -> String {
        let mut parts = vec![format!("launch options for {} game(s)", self.launch)];
        if self.proton > 0 {
            parts.push(format!("the Proton version for {} game(s)", self.proton));
        }
        format!("gameready can set {}. How?", parts.join(" and "))
    }
}

/// Asks which way to apply the settings.
///
/// Escaping the prompt means showing them rather than closing Steam: the
/// reversible, nothing-happens answer is the one an interrupted prompt should
/// land on.
pub fn choose_how_to_apply(work: &SteamWork) -> Result<LaunchChoice> {
    let answer = Select::new(
        &work.question(),
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
