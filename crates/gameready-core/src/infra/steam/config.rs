//! Finding the config file Steam keeps a user's per-game settings in.

use std::path::{Path, PathBuf};

use steamlocate::SteamDir;

use crate::steam::SteamError;

/// Where each Steam account's own settings live, one directory per account id.
const USERDATA: &str = "userdata";

/// The file inside an account's directory that holds launch options.
const LOCAL_CONFIG: &str = "config/localconfig.vdf";

/// The `localconfig.vdf` of the account most likely to be the one in use.
///
/// A machine can hold several accounts' directories, and nothing in the file
/// tree says which is logged in. The most recently written one is the account
/// that last used this machine, which is the best answer available without
/// asking Steam itself.
pub fn locate_local_config() -> Result<PathBuf, SteamError> {
    let steam = SteamDir::locate().map_err(|_| SteamError::NotInstalled)?;
    local_config_under(steam.path())
}

/// The same lookup under a Steam directory chosen by the caller.
pub fn local_config_under(steam_dir: &Path) -> Result<PathBuf, SteamError> {
    let listing =
        std::fs::read_dir(steam_dir.join(USERDATA)).map_err(|_| SteamError::NoUserConfig)?;

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in listing.flatten() {
        let candidate = entry.path().join(LOCAL_CONFIG);
        let Ok(modified) = candidate.metadata().and_then(|meta| meta.modified()) else {
            continue;
        };
        if newest.as_ref().is_none_or(|(seen, _)| modified > *seen) {
            newest = Some((modified, candidate));
        }
    }

    newest.map(|(_, path)| path).ok_or(SteamError::NoUserConfig)
}

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;
