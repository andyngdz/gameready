//! How a game is named and looked up.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A Steam application id.
///
/// A newtype rather than a bare `u32` because it crosses every layer of the
/// games feature and gets compared against numbers read out of manifest files.
/// Mixing it up with a user id or a depot id would be a silent wrong-game bug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppId(pub u32);

impl fmt::Display for AppId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The key a profile is filed and looked up under.
///
/// Derived from the display name: lowercased, and everything that is not a
/// letter or a digit collapsed to a single `-`. That makes `Cyberpunk 2077`,
/// `cyberpunk-2077`, and `Cyberpunk  2077` all reach the same profile, which
/// matters because the user types this on the command line and the catalog
/// layers are three separate directories that people name by hand.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GameKey(String);

impl GameKey {
    /// Builds the key for a display name.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let mut key = String::with_capacity(name.len());
        let mut pending_separator = false;

        for character in name.chars() {
            if character.is_ascii_alphanumeric() {
                if pending_separator && !key.is_empty() {
                    key.push('-');
                }
                pending_separator = false;
                key.push(character.to_ascii_lowercase());
            } else {
                pending_separator = true;
            }
        }
        Self(key)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for GameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a per-game step is acting on: the game, and which library entry it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRef {
    pub key: GameKey,
    pub name: String,
    pub app_id: AppId,
}

#[cfg(test)]
#[path = "identity_test.rs"]
mod identity_test;
