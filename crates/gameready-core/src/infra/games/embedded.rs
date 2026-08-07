//! The profiles compiled into the binary.

use std::path::PathBuf;

use rust_embed::Embed;

use crate::games::{parse_profile, GameError, GameProfile};

/// Everything under the repository's `games/` directory.
///
/// Embedded rather than installed alongside the binary so a single static
/// musl build works on a machine with no gameready package and no data
/// directory, which is how the release artifact is meant to be used.
///
/// The `debug-embed` feature is on so debug builds embed too. Without it
/// rust-embed reads from the source tree at runtime in debug, and every test
/// here would pass while proving nothing about the shipped binary.
#[derive(Embed)]
#[folder = "$CARGO_MANIFEST_DIR/../../games/"]
#[include = "*/game.toml"]
struct Builtin;

/// Reads every built-in profile.
///
/// Returns the failures alongside the successes rather than stopping at the
/// first one: a broken profile should cost the user that game, not the whole
/// catalog. Every one of these files ships in the binary, so a failure here is
/// a bug in this repository and the registry test catches it before release.
#[must_use]
pub fn builtin_profiles() -> (Vec<GameProfile>, Vec<GameError>) {
    let mut profiles = Vec::new();
    let mut failures = Vec::new();

    for name in Builtin::iter() {
        let path = PathBuf::from(name.as_ref());
        let Some(file) = Builtin::get(name.as_ref()) else {
            continue;
        };

        match std::str::from_utf8(file.data.as_ref()) {
            Ok(text) => match parse_profile(&path, text) {
                Ok(profile) => profiles.push(profile),
                Err(error) => failures.push(error),
            },
            Err(source) => failures.push(GameError::Read {
                path,
                source: std::io::Error::new(std::io::ErrorKind::InvalidData, source),
            }),
        }
    }
    (profiles, failures)
}

#[cfg(test)]
#[path = "embedded_test.rs"]
mod embedded_test;
