//! Putting the installed games and the profile catalog together.

use std::path::Path;

use crate::infra::games::load_catalog;
use crate::infra::steam::scan::scan_installed_games;
use crate::steam::{pair_with_catalog, GameSetup};

/// Every installed game, paired with the profile that matches its appid.
///
/// A Steam that is missing or unreadable yields an empty list rather than an
/// error. The core tuning is the larger half of what `init` does and needs no
/// Steam at all, so failing the whole run over the game list would take away
/// the part that still works.
#[must_use]
pub fn discover_setups(user_games_dir: &Path) -> Vec<GameSetup> {
    let (catalog, _) = load_catalog(user_games_dir);
    let installed = scan_installed_games().unwrap_or_default();
    pair_with_catalog(&installed, &catalog)
}

#[cfg(test)]
#[path = "setup_test.rs"]
mod setup_test;
