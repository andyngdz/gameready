//! Finding the config file Steam keeps a user's per-game settings in.

use std::path::{Path, PathBuf};

use steamlocate::SteamDir;

use crate::steam::SteamError;
use crate::steps::{COMPAT_TOOLS_DIR, COMPAT_TOOL_VDF};

/// The files Steam keeps the settings gameready writes in.
///
/// Two files rather than one: launch options are per account and live in
/// `localconfig.vdf`, while the Proton mapping is per machine and lives in
/// `config.vdf`.
#[derive(Debug, Clone)]
pub struct SteamConfigs {
    pub local: PathBuf,
    pub install: PathBuf,
}

/// Where each Steam account's own settings live, one directory per account id.
const USERDATA: &str = "userdata";

/// The file inside an account's directory that holds launch options.
const LOCAL_CONFIG: &str = "config/localconfig.vdf";

/// The machine-wide config, holding which Proton build runs which game.
const INSTALL_CONFIG: &str = "config/config.vdf";

/// Where Steam is installed.
///
/// The caller keeps it because everything else here hangs off it: the two
/// config files and the compatibility tools directory.
pub fn locate_steam_dir() -> Result<PathBuf, SteamError> {
    let steam = SteamDir::locate().map_err(|_| SteamError::NotInstalled)?;
    Ok(steam.path().to_path_buf())
}

/// Both config files gameready writes, under a given Steam directory.
pub fn configs_under(steam_dir: &Path) -> Result<SteamConfigs, SteamError> {
    Ok(SteamConfigs {
        local: local_config_under(steam_dir)?,
        install: install_config_under(steam_dir),
    })
}

/// The `localconfig.vdf` of the account most likely to be the one in use.
///
/// A machine can hold several accounts' directories, and nothing in the file
/// tree says which is logged in. The most recently written one is the account
/// that last used this machine, which is the best answer available without
/// asking Steam itself.
pub fn locate_local_config() -> Result<PathBuf, SteamError> {
    local_config_under(&locate_steam_dir()?)
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

/// Steam's machine-wide `config.vdf`, which holds the Proton mapping.
///
/// One file per installation rather than one per account, because which build
/// runs a game is a property of the machine.
#[must_use]
pub fn install_config_under(steam_dir: &Path) -> PathBuf {
    steam_dir.join(INSTALL_CONFIG)
}

/// The compatibility tools installed by hand, by their directory names.
///
/// These names are what Steam records in its mapping, so they are what a pin
/// has to be written with. An unreadable or absent directory reads as none
/// installed, which is what a machine that never added one looks like.
#[must_use]
pub fn installed_compat_tools(steam_dir: &Path) -> Vec<String> {
    let Ok(listing) = std::fs::read_dir(steam_dir.join(COMPAT_TOOLS_DIR)) else {
        return Vec::new();
    };

    listing
        .flatten()
        .filter(|entry| entry.path().join(COMPAT_TOOL_VDF).is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

#[cfg(test)]
#[path = "config_test.rs"]
mod config_test;
