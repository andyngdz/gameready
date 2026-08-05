//! What Steam has installed.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::games::AppId;

/// One entry in a Steam library that gameready is willing to treat as a game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledGame {
    pub app_id: AppId,

    /// The name Steam shows, which is also the name a user will look for.
    pub name: String,

    /// Where the files are. Kept for the per-game steps that need to reach into
    /// a game's directory, such as a shader cache.
    pub install_dir: PathBuf,
}

impl InstalledGame {
    #[must_use]
    pub const fn new(app_id: AppId, name: String, install_dir: PathBuf) -> Self {
        Self {
            app_id,
            name,
            install_dir,
        }
    }
}

#[cfg(test)]
#[path = "library_test.rs"]
mod library_test;
