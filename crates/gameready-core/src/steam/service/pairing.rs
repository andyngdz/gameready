//! Matching installed games against the profiles gameready ships.

use crate::games::{Catalog, GameProfile, Source, Wrapper, launch_options};
use crate::steam::domain::InstalledGame;

/// One installed game, and what gameready can do for it.
///
/// A game with no profile is kept rather than dropped. The core tuning applies
/// to every game on the machine, so silently leaving a game out of the list
/// would suggest gameready cannot help with it at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSetup {
    pub game: InstalledGame,

    /// The matching profile, when one exists.
    pub profile: Option<GameProfile>,

    /// Where that profile came from, so a user can see their own copy winning.
    pub source: Option<Source>,
}

impl GameSetup {
    /// Whether gameready has per-game settings for this one.
    #[must_use]
    pub const fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// The launch option string this game's profile asks for.
    ///
    /// Empty for a game with no profile, and for a profile that asks for
    /// nothing. Both mean "leave Steam's box alone" rather than "clear it".
    #[must_use]
    pub fn launch_options(&self) -> String {
        self.profile
            .as_ref()
            .map(launch_options)
            .unwrap_or_default()
    }
}

/// Whether the user asked for the frame-rate overlay.
///
/// A named answer rather than a bool because it is the user's decision and it
/// reads at every call site: `Overlay::Hide` says what happens, `false` does
/// not. Nothing turns it on implicitly; mangohud's default overlay covers a
/// corner of the screen with load, temperatures, and a frametime graph, which
/// is not something to put in front of someone who did not ask for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// Add mangohud to the launch options of every selected game.
    Show,
    /// Leave the launch options alone. The default.
    Hide,
}

impl Overlay {
    /// What an unanswered question means.
    ///
    /// Every path that cannot ask lands here: a scripted run, a skipped prompt,
    /// a terminal that cannot show one. Putting an overlay on someone's screen
    /// is not something to do because nobody said otherwise.
    #[must_use]
    pub const fn default_answer() -> Self {
        Self::Hide
    }
}

/// Applies the overlay answer to the selected games.
///
/// Only games with a profile are touched, because those are the only ones whose
/// launch options gameready writes at all. A game already asking for mangohud
/// is left as it is rather than gaining a second copy.
#[must_use]
pub fn with_overlay(setups: &[GameSetup], overlay: Overlay) -> Vec<GameSetup> {
    if overlay == Overlay::Hide {
        return setups.to_vec();
    }

    setups
        .iter()
        .map(|setup| {
            let mut setup = setup.clone();
            if let Some(profile) = setup.profile.as_mut()
                && !profile.wrappers.contains(&Wrapper::MangoHud)
            {
                // Appended, so it ends up innermost: gamemode and gamescope have
                // to wrap it, not the other way round.
                profile.wrappers.push(Wrapper::MangoHud);
            }
            setup
        })
        .collect()
}

/// The setups that have a profile, which is what `init` ticks by default.
///
/// Lives here rather than in the CLI so the rule "a run with no picker takes
/// exactly the games gameready has settings for" is one testable function
/// instead of a filter written at each call site.
#[must_use]
pub fn with_profiles(setups: &[GameSetup]) -> Vec<GameSetup> {
    setups
        .iter()
        .filter(|setup| setup.has_profile())
        .cloned()
        .collect()
}

/// Pairs every installed game with its profile, keeping the scan's order.
#[must_use]
pub fn pair_with_catalog(games: &[InstalledGame], catalog: &Catalog) -> Vec<GameSetup> {
    games
        .iter()
        .map(|game| {
            let entry = catalog.by_app_id(game.app_id);
            GameSetup {
                game: game.clone(),
                profile: entry.map(|entry| entry.profile.clone()),
                source: entry.map(|entry| entry.source),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "pairing_test.rs"]
mod pairing_test;
