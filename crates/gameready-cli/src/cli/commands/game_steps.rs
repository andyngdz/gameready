//! Building the per-game steps from what is actually installed.
//!
//! `init` builds these from the games the user picked in the picker. There is
//! no picker in `selftest`, so this takes the same route the picker's defaults
//! would: every installed game gameready has a profile for.

use std::path::Path;

use anyhow::{anyhow, Context as _, Result};
use gameready_core::improvement::{CoreImprovement, ImprovementId};
use gameready_core::infra::steam::{
    configs_under, discover_setups, installed_compat_tools, locate_steam_dir,
};
use gameready_core::run::{compat_wishes_for, targets_for};
use gameready_core::steps::{resolve_wishes, SteamLaunchOptions, SteamProton};

/// What to say when Steam's own files cannot be found.
const NO_STEAM: &str = "could not find a Steam installation to test against";

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
) -> Result<Box<dyn CoreImprovement>> {
    let steam = locate_steam_dir().context(NO_STEAM)?;
    let configs = configs_under(&steam).context(NO_STEAM)?;

    let setups = discover_setups(user_games_dir);
    if setups.is_empty() {
        return Err(anyhow!(
            "no installed game has a gameready profile, so there is nothing for \
             `{id}` to write"
        ));
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
