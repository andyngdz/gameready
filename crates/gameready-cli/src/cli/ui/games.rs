//! Rendering the game catalog.

use std::fmt;

use console::style;
use gameready_core::games::{Catalog, GameError, ProtonChoice};

use crate::cli::ui::layout::{Mark, Section};

/// The dim label shown before the wrappers a game launches through.
const LAUNCH_THROUGH: &str = "launch through";

/// The dim label shown before the Proton build a game runs under.
const RUN_UNDER: &str = "run under";

/// A note under the list explaining that a profile the user writes wins over a
/// shipped one, which is what the "yours" source label points at.
const PRECEDENCE_NOTE: &str =
    "Profiles you write live in ~/.config/gameready/games and win over the built-in ones.";

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

    /// Every game, each with its origin and the settings that change how it
    /// runs.
    fn games<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        let name_column = self
            .catalog
            .entries()
            .iter()
            .map(|entry| console::measure_text_width(&entry.profile.name))
            .max()
            .unwrap_or(0);
        let label_column = LAUNCH_THROUGH.len();

        for entry in self.catalog.entries() {
            let profile = &entry.profile;
            let origin = format!("{} · {}", profile.app_id, entry.source.label());
            s.entry(&profile.name, &origin, name_column)?;

            let wrappers: Vec<&str> = profile
                .wrappers
                .iter()
                .map(|wrapper| wrapper.command())
                .collect();
            if !wrappers.is_empty() {
                s.detail(LAUNCH_THROUGH, &wrappers.join(", "), label_column)?;
            }

            // Listed for the same reason as the wrappers: a profile that pins a
            // Proton version changes how the game runs, and a list that shows
            // one setting and hides the other reads as if there is only one.
            if let Some(proton) = &profile.proton {
                s.detail(RUN_UNDER, &describe(proton), label_column)?;
            }
            s.blank()?;
        }
        Ok(())
    }

    /// The profiles that could not be read, each with its path and the reason
    /// on its own line under it.
    fn unreadable<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        if self.failures.is_empty() {
            return Ok(());
        }
        s.heading(&style(count_files(self.failures.len())).yellow().to_string())?;
        for failure in self.failures {
            s.marked(Mark::Warning, &failure.path().display().to_string())?;
            s.sub(&failure.detail())?;
        }
        s.blank()?;
        Ok(())
    }
}

impl fmt::Display for GameList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.blank()?;
        s.title(&count_profiles(self.catalog.len()))?;
        self.games(&mut s)?;
        self.unreadable(&mut s)?;
        s.indented(&style(PRECEDENCE_NOTE).dim().to_string())
    }
}

/// The list heading, counting the profiles it is about.
fn count_profiles(count: usize) -> String {
    match count {
        0 => "No game profiles".to_owned(),
        1 => "1 game profile".to_owned(),
        n => format!("{n} game profiles"),
    }
}

/// The unreadable-files heading, counting the files that failed.
fn count_files(count: usize) -> String {
    match count {
        1 => "Couldn't read 1 file".to_owned(),
        n => format!("Couldn't read {n} files"),
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
