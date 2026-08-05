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

/// The wrappers a `[launch]` table asks for, outermost first.
///
/// The order is fixed here rather than taken from the file. gamemode has to be
/// outermost for its priority change to cover everything below it, and gamescope
/// has to sit outside the game but inside gamemode so the compositor is itself
/// prioritised. A profile that could reorder these would mostly reorder them
/// wrong.
fn wrappers(launch: &LaunchToml) -> Vec<Wrapper> {
    let mut wrappers = Vec::new();
    if launch.gamemode {
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
