//! Finding Steam and reading its libraries.

use std::path::Path;

use steamlocate::SteamDir;

use super::appinfo::NonGameApps;
use crate::games::AppId;
use crate::steam::{is_valve_tool, InstalledGame, SteamError};

/// Steam's index of where its library folders are. Its presence is what makes a
/// directory a Steam installation rather than any directory.
const LIBRARY_INDEX: &str = "steamapps/libraryfolders.vdf";

/// Where a library folder keeps the games themselves.
const COMMON: &str = "steamapps/common";

/// Every game installed through Steam, across all its library folders.
///
/// Non-game entries are dropped two ways: Valve's runtimes and
/// redistributables by name and appid (see [`crate::steam::is_valve_tool`]),
/// and anything Steam types as other than a game (tools, soundtracks, bonus
/// content), which [`NonGameApps`] reads out of `appinfo.vdf`. When that file
/// cannot be read the second filter is skipped rather than the scan failing.
pub fn scan_installed_games() -> Result<Vec<InstalledGame>, SteamError> {
    let steam = SteamDir::locate().map_err(|_| SteamError::NotInstalled)?;
    games_in(&steam)
}

/// The same scan against a Steam directory chosen by the caller.
///
/// The whole point is that a fixture tree can stand in for a real install, so
/// the filtering and ordering are covered by ordinary tests rather than only by
/// running this on a machine that happens to have the right games.
pub fn scan_installed_games_in(steam_dir: &Path) -> Result<Vec<InstalledGame>, SteamError> {
    // Checked here rather than left to the parser. `SteamDir::from_dir` accepts
    // any directory and only fails when the index is read, so without this a
    // path that is not a Steam install at all reports as a corrupt library.
    if !steam_dir.join(LIBRARY_INDEX).is_file() {
        return Err(SteamError::NotInstalled);
    }
    let steam = SteamDir::from_dir(steam_dir).map_err(|_| SteamError::NotInstalled)?;
    games_in(&steam)
}

fn games_in(steam: &SteamDir) -> Result<Vec<InstalledGame>, SteamError> {
    let libraries = steam
        .libraries()
        .map_err(|source| SteamError::UnreadableLibrary { source })?;

    let non_games = NonGameApps::read(steam.path());

    let mut games = Vec::new();
    for library in libraries.flatten() {
        for app in library.apps().flatten() {
            let app_id = AppId(app.app_id);
            // An app with no name is one Steam has half-registered, usually
            // mid-install. Nothing can be shown for it and nothing can be tuned.
            let Some(name) = app.name else {
                continue;
            };
            if is_valve_tool(app_id, &name) {
                continue;
            }
            if non_games.contains(app_id) {
                continue;
            }
            games.push(InstalledGame::new(
                app_id,
                name,
                library.path().join(COMMON).join(app.install_dir),
            ));
        }
    }

    // Sorted by name so the pick list reads the same on every run. Steam hands
    // apps back in whatever order the index has them, which is neither
    // alphabetical nor stable.
    games.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(games)
}

#[cfg(test)]
#[path = "scan_test.rs"]
mod scan_test;
