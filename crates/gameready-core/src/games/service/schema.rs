//! The exact shape of a `game.toml`.
//!
//! Kept separate from [`crate::games::GameProfile`] on purpose. This mirrors
//! the file, including the booleans a person wants to type; the profile is what
//! the rest of the code reasons about. Collapsing them would push
//! `gamemode: bool` into every consumer and make the file format impossible to
//! change without touching them all.

use std::collections::BTreeMap;

use serde::Deserialize;

/// One `game.toml`, straight off disk.
///
/// `deny_unknown_fields` throughout: a typo in a key would otherwise be
/// accepted in silence, and the user would be left wondering why the setting
/// they wrote did nothing.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameToml {
    pub name: String,
    pub steam_appid: u32,

    #[serde(default)]
    pub launch: LaunchToml,

    #[serde(default)]
    pub env: BTreeMap<String, String>,

    pub proton: Option<ProtonToml>,

    #[serde(rename = "override")]
    pub override_section: Option<OverrideToml>,
}

/// The `[launch]` table.
///
/// `%command%` is deliberately absent: gameready inserts it, so a profile
/// cannot put it in the wrong place or leave it out.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchToml {
    #[serde(default)]
    pub gamemode: bool,

    #[serde(default)]
    pub mangohud: bool,

    #[serde(default)]
    pub gamescope: bool,
}

/// The `[proton]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtonToml {
    pub prefer: String,
}

/// The `[override]` table, for a game a declarative profile cannot express.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverrideToml {
    pub module: String,
}
