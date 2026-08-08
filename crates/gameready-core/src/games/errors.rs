//! Why a profile could not be read.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// A profile that did not load.
///
/// Every variant names the file, because the catalog reads from three
/// directories and "a game.toml is broken" without a path is unactionable.
#[derive(Debug, Error)]
pub enum GameError {
    /// Bad TOML, or good TOML with a key the schema does not accept. Both come
    /// back from the parser as one error carrying the line and column, and the
    /// message it renders already says which of the two it was.
    #[error("`{path}` is not a usable game profile")]
    Invalid {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("`{path}` has an empty name, so it could never be looked up")]
    NoName { path: PathBuf },

    #[error("`{path}` sets steam_appid to 0, which is not a Steam application")]
    NoAppId { path: PathBuf },

    #[error("reading `{path}` failed")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl GameError {
    /// The file this failure is about. Every variant names one.
    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Invalid { path, .. }
            | Self::NoName { path }
            | Self::NoAppId { path }
            | Self::Read { path, .. } => path,
        }
    }

    /// The cause on its own, without the path, so a report can show the path on
    /// one line and the reason indented under it rather than repeating the path
    /// inside the sentence.
    #[must_use]
    pub fn detail(&self) -> String {
        match self {
            Self::Invalid { source, .. } => source.to_string(),
            Self::NoName { .. } => "it sets no name, so it could never be looked up".to_owned(),
            Self::NoAppId { .. } => "steam_appid is 0, which is not a Steam application".to_owned(),
            Self::Read { source, .. } => source.to_string(),
        }
    }
}
