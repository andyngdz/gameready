//! Every profile gameready can see, and where each one came from.

use std::collections::BTreeMap;

use crate::games::domain::{AppId, GameKey, GameProfile};

/// Where a profile was found.
///
/// Carried through to the output so a user who overrode a shipped profile can
/// see that their copy is the one in effect, which is otherwise invisible and
/// is the first thing to check when a profile "does nothing".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// Compiled into the binary.
    Builtin,
    /// `/usr/share/gameready/games`, put there by a package.
    System,
    /// `~/.config/gameready/games`, written by the user.
    User,
}

impl Source {
    /// How the source reads in the game list.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Builtin => "built in",
            Self::System => "system",
            Self::User => "yours",
        }
    }
}

/// One profile and where it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub profile: GameProfile,
    pub source: Source,
}

/// The profiles gameready can act on, keyed for lookup.
///
/// Layers are added in precedence order and a later one replaces an earlier
/// one outright rather than merging field by field. Merging would produce a
/// profile that exists in no file, so a user editing their copy could not
/// predict the result from what they wrote.
#[derive(Debug, Default, Clone)]
pub struct Catalog {
    entries: BTreeMap<GameKey, CatalogEntry>,
}

impl Catalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one layer, replacing anything already filed under the same key.
    ///
    /// Call in precedence order: built in, then system, then the user's.
    pub fn overlay(&mut self, source: Source, profiles: impl IntoIterator<Item = GameProfile>) {
        for profile in profiles {
            self.entries
                .insert(profile.key(), CatalogEntry { profile, source });
        }
    }

    /// The profile filed under a key, whichever layer won it.
    #[must_use]
    pub fn get(&self, key: &GameKey) -> Option<&CatalogEntry> {
        self.entries.get(key)
    }

    /// Looks a game up the way a user types it: by name, in any casing or
    /// punctuation.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&CatalogEntry> {
        self.get(&GameKey::from_name(name))
    }

    /// The profile for a Steam application, when one exists.
    ///
    /// Matched on the appid rather than the name. Steam's name for a game and
    /// the name in a profile drift apart over re-releases and editions, and the
    /// appid is what actually identifies the thing being tuned.
    #[must_use]
    pub fn by_app_id(&self, app_id: AppId) -> Option<&CatalogEntry> {
        self.entries
            .values()
            .find(|entry| entry.profile.app_id == app_id)
    }

    /// Every profile, in key order so the list is the same on every machine.
    #[must_use]
    pub fn entries(&self) -> Vec<&CatalogEntry> {
        self.entries.values().collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
#[path = "catalog_test.rs"]
mod catalog_test;
