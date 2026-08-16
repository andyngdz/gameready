//! Building the per-game steps from what is actually installed.
//!
//! `init` builds these from the games the user picked in the picker. There is
//! no picker in `selftest`, so this takes the same route the picker's defaults
//! would: every installed game gameready has a profile for.

use std::fmt;
use std::path::Path;

use gameready_core::improvement::{CoreImprovement, ImprovementId};
use gameready_core::infra::steam::{
    configs_under, discover_setups, installed_compat_tools, locate_steam_dir,
};
use gameready_core::run::{compat_wishes_for, targets_for};
use gameready_core::steps::{resolve_wishes, SteamLaunchOptions, SteamProton};

/// Why a per-game step could not be built.
///
/// A typed error so the selftest sweep can tell a missing machine from a broken
/// one: the first is a skip, the second has to fail or "all passed" would be a
/// lie about a step that was never really run.
#[derive(Debug)]
pub enum GameStepBuildError {
    /// No Steam installation to build the step against, or no account config
    /// under it yet.
    SteamUnavailable,
    /// No installed games to write the step to. The scan covers every game
    /// Steam has installed; a gameready profile only changes what gets written.
    NoGames { step: ImprovementId },
}

impl fmt::Display for GameStepBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SteamUnavailable => {
                f.write_str("could not find a Steam installation to test against")
            }
            Self::NoGames { step } => write!(
                f,
                "no installed games were found, so there is nothing for `{step}` to write"
            ),
        }
    }
}

impl std::error::Error for GameStepBuildError {}

/// Whether an id names one of the per-game steps.
#[must_use]
pub fn is_game_step(id: &ImprovementId) -> bool {
    *id == SteamLaunchOptions::id_const() || *id == SteamProton::id_const()
}

/// Builds one per-game step against the real Steam installation.
///
/// Errors rather than returning an empty step when Steam is missing. A step
/// built with no config path probes as not-applicable, which would report as a
/// skip and read as "nothing to test here" rather than "there was nothing to
/// test it against".
pub fn build_game_step(
    id: &ImprovementId,
    user_games_dir: &Path,
) -> Result<Box<dyn CoreImprovement>, GameStepBuildError> {
    let steam = locate_steam_dir().map_err(|_| GameStepBuildError::SteamUnavailable)?;
    let configs = configs_under(&steam).map_err(|_| GameStepBuildError::SteamUnavailable)?;

    let setups = discover_setups(user_games_dir);
    if setups.is_empty() {
        return Err(GameStepBuildError::NoGames { step: id.clone() });
    }

    if *id == SteamLaunchOptions::id_const() {
        return Ok(Box::new(SteamLaunchOptions::new(
            configs.local,
            targets_for(&setups),
        )));
    }

    let wishes = compat_wishes_for(&setups);
    let tools = installed_compat_tools(&steam);
    Ok(Box::new(SteamProton::new(
        configs.install,
        resolve_wishes(&wishes, &tools),
    )))
}

#[cfg(test)]
#[path = "game_steps_test.rs"]
mod game_steps_test;
