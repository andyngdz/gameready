//! Profiles a user or a package put on disk.

use std::path::{Path, PathBuf};

use crate::games::{parse_profile, GameError, GameProfile};

/// Where a packaged gameready drops its profiles.
pub const SYSTEM_GAMES_DIR: &str = "/usr/share/gameready/games";

/// The file inside each game's directory.
const PROFILE_FILE: &str = "game.toml";

/// Reads every `<dir>/<Game>/game.toml`.
///
/// A directory that does not exist is not an error. Two of the three catalog
/// layers are absent on a normal machine, and treating that as a failure would
/// make the common case print a warning about nothing.
#[must_use]
pub fn profiles_in(dir: &Path) -> (Vec<GameProfile>, Vec<GameError>) {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return (Vec::new(), Vec::new());
    };

    let mut profiles = Vec::new();
    let mut failures = Vec::new();

    for entry in listing.flatten() {
        let path = entry.path().join(PROFILE_FILE);
        if !path.is_file() {
            continue;
        }
        match read_profile(&path) {
            Ok(profile) => profiles.push(profile),
            Err(error) => failures.push(error),
        }
    }
    (profiles, failures)
}

fn read_profile(path: &PathBuf) -> Result<GameProfile, GameError> {
    let text = std::fs::read_to_string(path).map_err(|source| GameError::Read {
        path: path.clone(),
        source,
    })?;
    parse_profile(path, &text)
}

#[cfg(test)]
#[path = "disk_test.rs"]
mod disk_test;
