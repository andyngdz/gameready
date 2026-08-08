//! The first thing `init` prints: what this machine is, before anything moves.

use std::fmt;

use console::style;
use gameready_core::facts::SystemFacts;

use crate::cli::ui::layout::{Mark, ResultTable, Section};
use crate::cli::ui::{name_column, short_names};

/// The promise the whole probing phase rests on. It is the answer to the only
/// question someone running an unfamiliar tuning tool actually has.
const NOTHING_YET: &str = "Nothing changes until you say so.";

/// How the Steam line opens whenever Steam is where it should be.
const STEAM_FOUND: &str = "Steam found";

/// What the run found where the games live.
///
/// A count on its own could not say whether zero means "no Steam here" or
/// "Steam, but nothing installed", and those are different machines with
/// different reasons for the per-game steps sitting out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SteamGames {
    /// Steam is installed, with this many games a run could tune.
    Found(usize),

    /// No Steam on this machine.
    Missing,
}

impl SteamGames {
    /// The mark and the line this reads as.
    fn line(self) -> (Mark, &'static str, String) {
        match self {
            Self::Found(0) => (
                Mark::Skipped,
                STEAM_FOUND,
                "no games installed yet".to_owned(),
            ),
            Self::Found(1) => (Mark::Applied, STEAM_FOUND, "1 game I can tune".to_owned()),
            Self::Found(count) => (
                Mark::Applied,
                STEAM_FOUND,
                format!("{count} games I can tune"),
            ),
            Self::Missing => (
                Mark::Skipped,
                "No Steam here",
                "the per-game tunings have nothing to work on".to_owned(),
            ),
        }
    }
}

/// The machine, as two lines the user can check against what they know.
///
/// Printed before the probing sweep rather than after it, so the first thing on
/// screen is recognisable: a wrong distro or a missing Steam is worth stopping
/// for, and it is cheaper to notice now than at the summary.
pub struct LookingAtMachine<'a> {
    facts: &'a SystemFacts,
    games: SteamGames,
}

impl<'a> LookingAtMachine<'a> {
    #[must_use]
    pub const fn new(facts: &'a SystemFacts, games: SteamGames) -> Self {
        Self { facts, games }
    }

    /// Prints the screen to stderr, when there is somebody there to read it.
    ///
    /// Held to the same rule as the progress spinner it runs into: a piped or
    /// redirected run gets the report on stdout and nothing else, so a script
    /// parsing the output never has to strip a banner.
    pub fn show(facts: &'a SystemFacts, games: SteamGames) {
        if console::user_attended_stderr() {
            eprint!("{}", Self::new(facts, games));
        }
    }

    /// The kernel and the package manager, after the distro's own name for
    /// itself. The release string is the raw one: a user recognises their
    /// machine by `7.0.0-29-generic`, not by `7.0.0`.
    fn system(&self) -> String {
        format!(
            "kernel {} · {}",
            self.facts.kernel_release,
            self.facts.distro.package_manager()
        )
    }
}

impl fmt::Display for LookingAtMachine<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.blank()?;
        s.title(&format!(
            "{} {}",
            style("Looking at your machine.").bold(),
            style(NOTHING_YET).dim()
        ))?;
        let (mark, name, note) = self.games.line();
        // The catalog's column, not these two lines': the probe rows that
        // follow are printed one at a time against that same edge.
        let mut table = ResultTable::new(name_column(&short_names()));
        table.row(Mark::Applied, &self.facts.distro.name, &self.system());
        table.row(mark, name, &note);
        s.heading(&table.to_string())
    }
}

#[cfg(test)]
#[path = "looking_test.rs"]
mod looking_test;
