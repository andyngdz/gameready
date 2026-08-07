//! Turning a set of per-game Proton choices into one edited config.

use crate::games::AppId;
use crate::steam::{SetResult, VdfError, set_block};
use crate::steps::constants::{
    COMPAT_CONFIG_KEY, COMPAT_MAPPING_PATH, COMPAT_NAME_KEY, COMPAT_PRIORITY, COMPAT_PRIORITY_KEY,
};

/// One game's Proton version, as it should end up in Steam's config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatTarget {
    pub app_id: AppId,

    /// The name Steam shows, so the plan and the summary can name the game
    /// rather than an appid the user does not recognise.
    pub name: String,

    /// The tool's internal name: a directory in `compatibilitytools.d` for a
    /// community build, or one of Valve's own names such as
    /// `proton_experimental`.
    pub tool: String,
}

/// The config after every target was applied, and what each one displaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatEdited {
    pub text: String,

    /// The targets that actually changed something, each with the tool it
    /// replaced. Empty means the config already pins what was asked for.
    pub replaced: Vec<(CompatTarget, String)>,
}

impl CompatEdited {
    /// Whether a given game still needs writing.
    #[must_use]
    pub fn is_pending(&self, app_id: AppId) -> bool {
        self.replaced
            .iter()
            .any(|(target, _)| target.app_id == app_id)
    }
}

/// Applies every target to `text` in one pass.
///
/// One pass over a single string so the file is written once. A write per game
/// would leave the config half updated if the run were interrupted between two
/// of them.
pub fn apply_compat_targets(
    text: &str,
    targets: &[CompatTarget],
) -> Result<CompatEdited, VdfError> {
    let mut current = text.to_owned();
    let mut replaced = Vec::new();

    for target in targets {
        let mut path: Vec<String> = COMPAT_MAPPING_PATH
            .iter()
            .map(|part| (*part).to_owned())
            .collect();
        path.push(target.app_id.to_string());
        let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();

        // Steam writes all three keys for every entry it owns. Writing only the
        // name leaves an entry Steam re-renders with the other two on its next
        // exit, which reads as gameready's change being partly undone.
        let values = [
            (COMPAT_NAME_KEY, target.tool.as_str()),
            (COMPAT_CONFIG_KEY, ""),
            (COMPAT_PRIORITY_KEY, COMPAT_PRIORITY),
        ];

        match set_block(&current, &borrowed, &values, COMPAT_NAME_KEY)? {
            SetResult::AlreadySet => {}
            SetResult::Changed(edit) => {
                replaced.push((target.clone(), edit.previous));
                current = edit.text;
            }
        }
    }

    Ok(CompatEdited {
        text: current,
        replaced,
    })
}

#[cfg(test)]
#[path = "compat_test.rs"]
mod compat_test;
