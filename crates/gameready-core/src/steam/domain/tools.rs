//! Telling Valve's plumbing apart from the games.
//!
//! A Steam library holds compatibility tools and runtimes next to the games,
//! and they are indistinguishable by anything structural: they are ordinary
//! installed apps with an appid, a name, and a directory. Left in, they fill
//! the "pick a game" list with entries nobody wants to tune.

use crate::games::AppId;

/// The runtimes and redistributables Valve ships as library entries.
///
/// Read off this machine's library on 2026-08-05. The list is not exhaustive
/// and cannot be: Valve mints a new appid for every Proton release. It exists
/// to catch the ones whose names do not follow the patterns below, and
/// [`TOOL_NAME_PREFIXES`] catches the rest.
const TOOL_APP_IDS: [u32; 7] = [
    228_980,   // Steamworks Common Redistributables
    1_070_560, // Steam Linux Runtime 1.0 (scout)
    1_391_110, // Steam Linux Runtime 2.0 (soldier)
    1_628_350, // Steam Linux Runtime 3.0 (sniper)
    4_183_110, // Steam Linux Runtime 4.0
    1_493_710, // Proton Experimental
    1_887_720, // Proton 7.0
];

/// Name prefixes that mark an entry as plumbing rather than a game.
///
/// Carries the whole family, so a Proton release published after this was
/// written is filtered without a code change. The trade is that a game named
/// "Proton something" would be hidden; no such game exists, and the appid list
/// above is not enough on its own.
const TOOL_NAME_PREFIXES: [&str; 4] = ["Proton", "Steam Linux Runtime", "Steamworks", "SteamVR"];

/// Whether this library entry is Valve plumbing rather than something to play.
///
/// Deliberately conservative. An extra entry in the list is noise the user
/// scrolls past; a missing game is a bug they cannot work around, so anything
/// this is unsure about is left in.
#[must_use]
pub fn is_valve_tool(app_id: AppId, name: &str) -> bool {
    if TOOL_APP_IDS.contains(&app_id.0) {
        return true;
    }
    TOOL_NAME_PREFIXES
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
#[path = "tools_test.rs"]
mod tools_test;
