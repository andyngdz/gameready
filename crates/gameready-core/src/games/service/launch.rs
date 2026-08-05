//! Building the string that goes in Steam's launch options box.

use crate::games::domain::{GameProfile, Wrapper};

/// The token Steam replaces with the game's own command line.
///
/// gameready inserts it rather than letting a profile write it, so it cannot
/// end up in the wrong place or be left out. A launch option without it makes
/// Steam run the wrappers and never the game.
const COMMAND: &str = "%command%";

/// Renders a profile as one Steam launch option string.
///
/// Order is fixed and load bearing: environment assignments first because a
/// shell applies them to everything that follows, then the wrappers outermost
/// to innermost, then the game. Empty when the profile asks for nothing, which
/// the caller reads as "leave the box alone" rather than "clear it".
#[must_use]
pub fn launch_options(profile: &GameProfile) -> String {
    let mut parts: Vec<String> = profile
        .env
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect();

    for wrapper in &profile.wrappers {
        parts.push(wrapper.command().to_owned());
        // gamescope takes its own flags, so without a separator it swallows
        // everything after it as arguments to itself and the game never starts.
        if matches!(wrapper, Wrapper::Gamescope) {
            parts.push("--".to_owned());
        }
    }

    if parts.is_empty() {
        return String::new();
    }

    parts.push(COMMAND.to_owned());
    parts.join(" ")
}

#[cfg(test)]
#[path = "launch_test.rs"]
mod launch_test;
