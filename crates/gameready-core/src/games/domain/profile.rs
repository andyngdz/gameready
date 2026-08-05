//! What one `game.toml` says.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::games::domain::identity::{AppId, GameKey, GameRef};

/// Everything gameready knows about how to run one game.
///
/// Built from a `game.toml`, never written by hand in Rust. Adding a game is
/// adding a file; a game that needs logic no profile can express points at a
/// module through [`GameProfile::override_module`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameProfile {
    /// What the game is called, as it appears in Steam.
    pub name: String,

    /// The Steam library entry this applies to.
    pub app_id: AppId,

    /// Wrappers to put in front of the game's command line, in the order they
    /// nest.
    pub wrappers: Vec<Wrapper>,

    /// Environment variables to set for the game. Ordered so a rendered launch
    /// string is the same on every machine, which is what makes it snapshot
    /// testable.
    pub env: BTreeMap<String, String>,

    /// Which Proton build to run under, when the profile has an opinion.
    pub proton: Option<ProtonChoice>,

    /// A Rust module that takes over for this game, for the cases a declarative
    /// profile cannot express.
    pub override_module: Option<String>,
}

impl GameProfile {
    /// The key this profile is filed under.
    #[must_use]
    pub fn key(&self) -> GameKey {
        GameKey::from_name(&self.name)
    }

    /// What a per-game step is handed to act on.
    #[must_use]
    pub fn game_ref(&self) -> GameRef {
        GameRef {
            key: self.key(),
            name: self.name.clone(),
            app_id: self.app_id,
        }
    }
}

/// A program the game is launched through.
///
/// An enum rather than three booleans because the order they nest in is part
/// of the meaning: `gamemoderun` on the outside, the game on the inside, and
/// getting that backwards produces a launch string that silently does nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrapper {
    /// Raises the governor and scheduling priority for this process tree.
    GameMode,
    /// Draws the frame rate overlay.
    MangoHud,
    /// Runs the game in its own nested compositor.
    Gamescope,
}

impl Wrapper {
    /// The executable that goes on the command line.
    ///
    /// gamemode's is `gamemoderun`, not `gamemode`: the package installs a
    /// daemon, a library, and this launcher script, and only the script belongs
    /// in a launch option.
    #[must_use]
    pub const fn command(self) -> &'static str {
        match self {
            Self::GameMode => "gamemoderun",
            Self::MangoHud => "mangohud",
            Self::Gamescope => "gamescope",
        }
    }
}

/// Which Proton build a profile asks for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "proton", rename_all = "snake_case")]
pub enum ProtonChoice {
    /// The newest GE-Proton in `compatibilitytools.d`, whatever that is today.
    NewestGeProton,

    /// Valve's Proton Experimental.
    Experimental,

    /// One exact tool name, used as written. For a game that regressed on a
    /// later build and has to stay put.
    Pinned { tool: String },
}

impl ProtonChoice {
    /// Reads the `prefer` value from a profile.
    ///
    /// Anything that is not one of the two names gameready resolves for itself
    /// is taken as an exact tool name, so a profile can pin a build that did
    /// not exist when this code was written.
    #[must_use]
    pub fn parse(prefer: &str) -> Self {
        match prefer {
            "GE-Proton" => Self::NewestGeProton,
            "Experimental" => Self::Experimental,
            tool => Self::Pinned {
                tool: tool.to_owned(),
            },
        }
    }
}

#[cfg(test)]
#[path = "profile_test.rs"]
mod profile_test;
