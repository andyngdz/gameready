//! Turning a set of per-game launch options into one edited config.

use crate::games::AppId;
use crate::steam::{capture_block, set_scalar, PriorSection, SetResult, VdfError};
use crate::steps::constants::{LAUNCH_OPTIONS_KEY, STEAM_APPS_PATH};

/// One game's launch options, as they should end up in Steam's config.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchTarget {
    pub app_id: AppId,

    /// The name Steam shows, so the plan and the summary can name the game
    /// rather than an appid the user does not recognise.
    pub name: String,

    pub options: String,
}

/// The config after every target was applied, and what each one displaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    pub text: String,

    /// The targets that actually changed something, each with the value it
    /// replaced. Empty means the config already says what was asked for.
    pub replaced: Vec<(LaunchTarget, String)>,
}

impl Edited {
    /// Whether a given game still needs writing.
    #[must_use]
    pub fn is_pending(&self, app_id: AppId) -> bool {
        self.replaced
            .iter()
            .any(|(target, _)| target.app_id == app_id)
    }
}

/// Where one game's own block sits in the config.
#[must_use]
pub fn app_section(app_id: AppId) -> Vec<String> {
    let mut path: Vec<String> = STEAM_APPS_PATH
        .iter()
        .map(|part| (*part).to_owned())
        .collect();
    path.push(app_id.to_string());
    path
}

/// Applies every target to `text` in one pass.
///
/// One pass over a single string so the file is written once. A write per game
/// would leave the config half updated if the run were interrupted between two
/// of them.
pub fn apply_targets(text: &str, targets: &[LaunchTarget]) -> Result<Edited, VdfError> {
    let mut current = text.to_owned();
    let mut replaced = Vec::new();

    for target in targets {
        let path = app_section(target.app_id);
        let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();

        match set_scalar(&current, &borrowed, LAUNCH_OPTIONS_KEY, &target.options)? {
            SetResult::AlreadySet => {}
            SetResult::Changed(edit) => {
                replaced.push((target.clone(), edit.previous));
                current = edit.text;
            }
        }
    }

    Ok(Edited {
        text: current,
        replaced,
    })
}

/// What every target's block held before the run wrote into it.
///
/// Captured from the text as it stands, before any edit, so the undo names what
/// was really there.
pub fn capture_targets(
    text: &str,
    targets: &[(LaunchTarget, String)],
) -> Result<Vec<PriorSection>, VdfError> {
    targets
        .iter()
        .map(|(target, _)| {
            let section = app_section(target.app_id);
            let borrowed: Vec<&str> = section.iter().map(String::as_str).collect();
            let prior = capture_block(text, &borrowed, &[LAUNCH_OPTIONS_KEY])?;
            Ok(PriorSection { section, prior })
        })
        .collect()
}

#[cfg(test)]
#[path = "launch_test.rs"]
mod launch_test;
