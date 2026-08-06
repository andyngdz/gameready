//! Turning one `game.toml` into a profile.

use std::path::Path;

use crate::games::domain::{GameProfile, ProtonChoice, Wrapper};
use crate::games::errors::GameError;
use crate::games::service::schema::{GameToml, LaunchToml};

/// Reads one profile.
///
/// `path` is carried only so an error can name the file. Nothing is read from
/// disk here, which is what lets every rule about the format be tested against
/// a string literal rather than a fixture tree.
pub fn parse_profile(path: &Path, text: &str) -> Result<GameProfile, GameError> {
    let raw: GameToml = toml::from_str(text).map_err(|source| GameError::Invalid {
        path: path.to_path_buf(),
        source,
    })?;

    if raw.name.trim().is_empty() {
        return Err(GameError::NoName {
            path: path.to_path_buf(),
        });
    }
    if raw.steam_appid == 0 {
        return Err(GameError::NoAppId {
            path: path.to_path_buf(),
        });
    }

    Ok(GameProfile {
        name: raw.name.trim().to_owned(),
        app_id: crate::games::domain::AppId(raw.steam_appid),
        wrappers: wrappers(&raw.launch),
        env: raw.env,
        proton: raw
            .proton
            .map(|proton| ProtonChoice::parse(proton.prefer.trim())),
        override_module: raw.override_section.map(|section| section.module),
    })
}

/// The wrappers every game gets when no profile says otherwise.
///
/// Defined as the empty `[launch]` table rather than a literal list, so the
/// defaults are whatever [`wrappers`] does with an unset field and there is only
/// one place to read to know what a game gets.
#[must_use]
pub fn default_wrappers() -> Vec<Wrapper> {
    wrappers(&LaunchToml::default())
}

/// The wrappers a `[launch]` table asks for, outermost first.
///
/// The order is fixed here rather than taken from the file. gamemode has to be
/// outermost for its priority change to cover everything below it, and gamescope
/// has to sit outside the game but inside gamemode so the compositor is itself
/// prioritised. A profile that could reorder these would mostly reorder them
/// wrong.
///
/// gamemode defaults to on because it helps every game and costs nothing when
/// it cannot: it raises the process priority for as long as the game runs and
/// does nothing at all when the daemon is missing.
fn wrappers(launch: &LaunchToml) -> Vec<Wrapper> {
    let mut wrappers = Vec::new();
    if launch.gamemode.unwrap_or(true) {
        wrappers.push(Wrapper::GameMode);
    }
    if launch.gamescope {
        wrappers.push(Wrapper::Gamescope);
    }
    if launch.mangohud {
        wrappers.push(Wrapper::MangoHud);
    }
    wrappers
}

#[cfg(test)]
#[path = "parse_test.rs"]
mod parse_test;
