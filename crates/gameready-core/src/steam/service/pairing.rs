//! Matching installed games against the profiles gameready ships.

use std::collections::BTreeMap;

use crate::games::{default_wrappers, launch_options, Catalog, GameProfile, Source, Wrapper};
use crate::steam::domain::InstalledGame;

/// One installed game, and what gameready will do for it.
///
/// Every game carries a profile, because every game gets the defaults. Whether
/// a `game.toml` matched is carried by `source` instead, which is the only
/// thing that actually differs between a tuned game and an untuned one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameSetup {
    pub game: InstalledGame,

    /// The settings in effect: a matched profile, or the defaults.
    pub profile: GameProfile,

    /// Where the profile came from, and `None` when these are the defaults.
    ///
    /// Doubles as the answer to "did a file match", so a user who overrode a
    /// shipped profile can see their own copy winning.
    pub source: Option<Source>,
}

impl GameSetup {
    /// Whether a `game.toml` matched, as opposed to taking the defaults.
    #[must_use]
    pub const fn has_profile(&self) -> bool {
        self.source.is_some()
    }

    /// The launch option string this game should end up with.
    ///
    /// Empty when the profile asks for nothing, which the caller reads as
    /// "leave Steam's box alone" rather than "clear it". A profile that turns
    /// every default off lands here too.
    #[must_use]
    pub fn launch_options(&self) -> String {
        launch_options(&self.profile)
    }
}

/// The settings a game gets when no `game.toml` matched it.
fn defaults_for(game: &InstalledGame) -> GameProfile {
    GameProfile {
        name: game.name.clone(),
        app_id: game.app_id,
        wrappers: default_wrappers(),
        env: BTreeMap::new(),
        proton: None,
        override_module: None,
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
/// A game already asking for mangohud is left as it is rather than gaining a
/// second copy.
#[must_use]
pub fn with_overlay(setups: &[GameSetup], overlay: Overlay) -> Vec<GameSetup> {
    if overlay == Overlay::Hide {
        return setups.to_vec();
    }

    setups
        .iter()
        .map(|setup| {
            let mut setup = setup.clone();
            if !setup.profile.wrappers.contains(&Wrapper::MangoHud) {
                // Appended, so it ends up innermost: gamemode and gamescope have
                // to wrap it, not the other way round.
                setup.profile.wrappers.push(Wrapper::MangoHud);
            }
            setup
        })
        .collect()
}

/// Pairs every installed game with the settings it will get, keeping the scan's
/// order.
///
/// A game with no matching `game.toml` gets the defaults rather than nothing,
/// which is what makes gameready useful on a library it has never seen.
#[must_use]
pub fn pair_with_catalog(games: &[InstalledGame], catalog: &Catalog) -> Vec<GameSetup> {
    games
        .iter()
        .map(|game| {
            let entry = catalog.by_app_id(game.app_id);
            GameSetup {
                game: game.clone(),
                profile: entry.map_or_else(|| defaults_for(game), |entry| entry.profile.clone()),
                source: entry.map(|entry| entry.source),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "pairing_test.rs"]
mod pairing_test;
