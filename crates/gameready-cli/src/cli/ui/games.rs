//! Rendering the game catalog.

use std::fmt;

use gameready_core::games::{Catalog, GameError, ProtonChoice};

use crate::cli::ui::layout::{Mark, Section};

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
        let mut s = Section::new(f);
        s.blank()?;
        s.title("Games")?;

        if self.catalog.is_empty() {
            s.indented(crate::cli::ui::NOTHING.trim_start())?;
        }

        for entry in self.catalog.entries() {
            let profile = &entry.profile;
            let origin = format!("{}  {}", profile.app_id, entry.source.label());
            s.row(Mark::None, &profile.name, Some(&origin))?;

            let wrappers: Vec<&str> = profile
                .wrappers
                .iter()
                .map(|wrapper| wrapper.command())
                .collect();
            if !wrappers.is_empty() {
                s.sub(&format!("launch through {}", wrappers.join(" ")))?;
            }

            // Listed for the same reason as the wrappers: a profile that pins a
            // Proton version changes how the game runs, and a list that shows
            // one setting and hides the other reads as if there is only one.
            if let Some(proton) = &profile.proton {
                s.sub(&format!("run under {}", describe(proton)))?;
            }
        }

        if !self.failures.is_empty() {
            s.blank()?;
            s.title("Could not read")?;
            for failure in self.failures {
                s.marked(Mark::Warning, &failure.to_string())?;
            }
        }

        Ok(())
    }
}

/// What a Proton choice reads as in a list, before anything resolves it.
///
/// The newest build cannot be named here: which one that is depends on what is
/// installed, and the catalog is read without touching a Steam directory.
fn describe(choice: &ProtonChoice) -> String {
    match choice {
        ProtonChoice::NewestGeProton => "the newest GE-Proton installed".to_owned(),
        ProtonChoice::Experimental => "Proton Experimental".to_owned(),
        ProtonChoice::Pinned { tool } => tool.clone(),
    }
}

#[cfg(test)]
#[path = "games_test.rs"]
mod games_test;
