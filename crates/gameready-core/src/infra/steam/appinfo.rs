//! Reading Steam's `appinfo.vdf` to tell games from everything else.
//!
//! A library holds more than games: soundtracks, bonus content, videos,
//! compatibility tools. Several of them get their own entry that is identical
//! to a game in every field the manifest carries: same `StateFlags`, same
//! `installdir`, same `SizeOnDisk`. The only local file that records what an
//! app actually *is* lives under `appcache`, a binary blob keyed by appid whose
//! `common/type` reads `Game`, `Tool`, `Music`, `DLC`, and so on. Read it, and
//! a soundtrack stops landing in the pick list with launch options it can never
//! use.

use std::collections::BTreeSet;
use std::path::Path;

use steam_vdf_parser::parse_appinfo;

use crate::games::AppId;

/// Where Steam caches per-app metadata, relative to the Steam root.
const APPINFO: &str = "appcache/appinfo.vdf";

/// The `common/type` value Steam writes for an app you launch and play.
///
/// Matched without case. Everything else Steam records (Tool, Music, Video,
/// DLC, Demo, Application) is not a game to tune, and is left out of the scan.
const GAME_TYPE: &str = "game";

/// The appids Steam types as something other than a game.
///
/// Built once per scan from `appinfo.vdf`. The scan drops these, because they
/// are otherwise indistinguishable from games and would be shown with launch
/// options they never use.
#[derive(Debug, Default)]
pub(crate) struct NonGameApps {
    ids: BTreeSet<u32>,
}

impl NonGameApps {
    /// The non-game appids recorded under this Steam root.
    ///
    /// Degrades to an empty set on any trouble: a missing file, a read error,
    /// or a format version the parser does not know because Steam bumped it. An
    /// empty set drops nothing, so the scan falls back to listing every app
    /// rather than failing. Getting this read wrong must never hide a game.
    #[must_use]
    pub(crate) fn read(steam_root: &Path) -> Self {
        let path = steam_root.join(APPINFO);
        let Ok(data) = std::fs::read(&path) else {
            return Self::default();
        };
        match Self::from_bytes(&data) {
            Some(apps) => apps,
            None => {
                tracing::debug!(
                    path = %path.display(),
                    "appinfo.vdf did not parse; non-game entries will not be filtered"
                );
                Self::default()
            }
        }
    }

    /// Parses the appinfo bytes into the set of non-game appids.
    ///
    /// Returns `None` when the blob is not appinfo Steam wrote in a format the
    /// parser understands; the caller turns that into an empty set. Only an app
    /// whose type is present and reads as something other than a game is
    /// recorded, so an app the file says nothing about is left for the scan.
    fn from_bytes(data: &[u8]) -> Option<Self> {
        let vdf = parse_appinfo(data).ok()?;
        let root = vdf.as_obj()?;
        let mut ids = BTreeSet::new();
        for (app_id_str, app_value) in root.iter() {
            let Ok(app_id) = app_id_str.parse::<u32>() else {
                continue;
            };
            let is_non_game = app_value
                .as_obj()
                .and_then(|app| app.get("appinfo"))
                .and_then(|appinfo| appinfo.as_obj())
                .and_then(|appinfo| appinfo.get("common"))
                .and_then(|common| common.as_obj())
                .and_then(|common| common.get("type"))
                .and_then(|type_value| type_value.as_str())
                .is_some_and(|type_name| !type_name.eq_ignore_ascii_case(GAME_TYPE));
            if is_non_game {
                ids.insert(app_id);
            }
        }
        Some(Self { ids })
    }

    /// Whether Steam types this appid as something other than a game.
    #[must_use]
    pub(crate) fn contains(&self, app_id: AppId) -> bool {
        self.ids.contains(&app_id.0)
    }
}

#[cfg(test)]
#[path = "appinfo_test.rs"]
mod appinfo_test;
