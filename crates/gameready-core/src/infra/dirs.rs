//! The per-user directories gameready reads and writes.
//!
//! Shared rather than derived per crate: the CLI writes the journal and the
//! profiles, the tray watches both, and a name that drifted between them would
//! leave the tray watching a file no run ever touches.
//!
//! Every function returns `None` for the one failure the platform has, a home
//! directory that cannot be resolved, and leaves what to do about it to the
//! caller: the CLI turns it into an error, the tray carries on without it.

use std::path::PathBuf;

use directories::ProjectDirs;

/// The name every per-user directory is built from.
const PROJECT: &str = "gameready";

/// Where a user's own game profiles live, under their config directory.
const GAMES: &str = "games";

/// The XDG directories for this project.
///
/// The two empty arguments are the qualifier and the organization, which only
/// macOS and Windows use to build a bundle id. On Linux `directories` discards
/// them and builds every path from the application name alone.
fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", PROJECT)
}

/// Where the journal, backups, and logs live.
///
/// Falls back to the data directory because `state_dir` is `None` on platforms
/// with no XDG state directory, and this data has to land somewhere.
#[must_use]
pub fn state_dir() -> Option<PathBuf> {
    let dirs = project_dirs()?;
    Some(
        dirs.state_dir()
            .unwrap_or_else(|| dirs.data_dir())
            .to_path_buf(),
    )
}

/// Where this user's own game profiles live.
///
/// Separate from the state directory: profiles are configuration a user writes
/// and keeps, while the state directory is data gameready writes and prunes.
#[must_use]
pub fn user_games_dir() -> Option<PathBuf> {
    Some(project_dirs()?.config_dir().join(GAMES))
}

#[cfg(test)]
#[path = "dirs_test.rs"]
mod dirs_test;
