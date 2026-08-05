//! Rendering the game catalog.

use std::fmt;

use gameready_core::games::{Catalog, GameError};

/// The catalog as printable lines, with anything that failed to load after it.
///
/// The source column is not decoration: a user who copied a shipped profile
/// into their own directory and edited it has no other way to confirm that
/// their copy is the one in effect.
pub struct GameList<'a> {
    catalog: &'a Catalog,
    failures: &'a [GameError],
}

impl<'a> GameList<'a> {
    #[must_use]
    pub const fn new(catalog: &'a Catalog, failures: &'a [GameError]) -> Self {
        Self { catalog, failures }
    }
}

impl fmt::Display for GameList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\nGames")?;

        if self.catalog.is_empty() {
            writeln!(f, "  none")?;
        }

        for entry in self.catalog.entries() {
            let profile = &entry.profile;
            writeln!(
                f,
                "  {:<28} {:>8}  {}",
                profile.name,
                profile.app_id,
                entry.source.label(),
            )?;

            let wrappers: Vec<&str> = profile
                .wrappers
                .iter()
                .map(|wrapper| wrapper.command())
                .collect();
            if !wrappers.is_empty() {
                writeln!(f, "  {:<28} launch through {}", "", wrappers.join(" "))?;
            }
        }

        if !self.failures.is_empty() {
            writeln!(f, "\nCould not read")?;
            for failure in self.failures {
                writeln!(f, "  ! {failure}")?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "games_test.rs"]
mod games_test;
