//! Build per-game steps from the installed Steam library.

use std::path::Path;

use gameready_core::improvement::{CoreImprovement, ImprovementId};
use gameready_core::infra::steam::{
    configs_under, discover_setups, installed_compat_tools, locate_steam_dir,
};
use gameready_core::run::{compat_wishes_for, targets_for};
use gameready_core::steps::{resolve_wishes, SteamLaunchOptions, SteamProton};

use super::errors::GameStepBuildError;

/// Whether an id names one of the per-game steps.
#[must_use]
pub(crate) fn is_game_step(id: &ImprovementId) -> bool {
    *id == SteamLaunchOptions::id_const() || *id == SteamProton::id_const()
}

/// Builds one per-game step against the real Steam installation.
pub(crate) fn build_game_step(
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
#[path = "service_test.rs"]
mod service_test;
