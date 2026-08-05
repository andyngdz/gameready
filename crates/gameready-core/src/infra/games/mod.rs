//! Loading the game catalog from the binary and from disk.

mod disk;
mod embedded;

use std::path::Path;

use crate::games::{Catalog, GameError, Source};

pub use disk::SYSTEM_GAMES_DIR;

/// Builds the catalog from all three layers, lowest precedence first.
///
/// `user_dir` is where the user's own profiles live, normally
/// `~/.config/gameready/games`. It is passed in rather than resolved here so a
/// test can point it at a temporary directory without touching a real home.
///
/// Failures come back alongside the catalog. One unreadable profile costs the
/// user that game, not the whole list.
#[must_use]
pub fn load_catalog(user_dir: &Path) -> (Catalog, Vec<GameError>) {
    let mut catalog = Catalog::new();
    let mut failures = Vec::new();

    let (builtin, builtin_failures) = embedded::builtin_profiles();
    catalog.overlay(Source::Builtin, builtin);
    failures.extend(builtin_failures);

    let (system, system_failures) = disk::profiles_in(Path::new(SYSTEM_GAMES_DIR));
    catalog.overlay(Source::System, system);
    failures.extend(system_failures);

    let (user, user_failures) = disk::profiles_in(user_dir);
    catalog.overlay(Source::User, user);
    failures.extend(user_failures);

    (catalog, failures)
}

#[cfg(test)]
#[path = "mod_test.rs"]
mod mod_test;
