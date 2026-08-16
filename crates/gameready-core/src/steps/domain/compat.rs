//! Turning a set of per-game Proton choices into one edited config.

use crate::games::{AppId, ProtonChoice};
use crate::steam::{capture_block, set_block, PriorSection, SetResult, VdfError};
use crate::steps::constants::{
    COMPAT_CONFIG_KEY, COMPAT_GAME_PRIORITY, COMPAT_MACHINE_WIDE_APP_ID,
    COMPAT_MACHINE_WIDE_PRIORITY, COMPAT_MAPPING_PATH, COMPAT_NAME_KEY, COMPAT_PRIORITY_KEY,
    PROTON_EXPERIMENTAL,
};

use super::proton_ge::newest_ge_proton;

/// What the machine-wide entry is called on the screens that list it.
///
/// Steam has no name for appid `0`, and "0" on a plan screen reads as a bug.
const MACHINE_WIDE_NAME: &str = "All other games";

/// Which of Steam's two mapping ranks an entry is written at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatRank {
    /// One game's own entry, which beats the machine-wide default.
    Game,

    /// The "run everything through this" entry, under appid `0`.
    MachineWide,
}

impl CompatRank {
    /// The priority Steam files an entry of this rank under.
    #[must_use]
    pub const fn priority(self) -> &'static str {
        match self {
            Self::Game => COMPAT_GAME_PRIORITY,
            Self::MachineWide => COMPAT_MACHINE_WIDE_PRIORITY,
        }
    }
}

/// A Proton choice, before it is resolved against the builds on this machine.
///
/// Separate from [`CompatTarget`] because the two are known at different
/// moments. A run can count its wishes while it is still asking questions, and
/// can only name the build once the step that installs Proton-GE has finished:
/// resolving early is how a run installs one build and pins the games to
/// another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatWish {
    pub app_id: AppId,

    /// The name Steam shows, so the plan and the summary can name the game
    /// rather than an appid the user does not recognise.
    pub name: String,

    /// The build the profile asked for, in the profile's own terms.
    pub choice: ProtonChoice,

    pub rank: CompatRank,
}

impl CompatWish {
    /// The wish for Steam's machine-wide default.
    ///
    /// Always the newest GE-Proton. A user who asked for the new build to be
    /// used asked for the new build, and an exact version here would go stale
    /// the next time one is installed.
    #[must_use]
    pub fn machine_wide() -> Self {
        Self {
            app_id: AppId(COMPAT_MACHINE_WIDE_APP_ID),
            name: MACHINE_WIDE_NAME.to_owned(),
            choice: ProtonChoice::NewestGeProton,
            rank: CompatRank::MachineWide,
        }
    }
}

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

    pub rank: CompatRank,
}

/// Every wish that resolves against the builds installed, as an entry to write.
///
/// A wish whose build is not there is dropped rather than written. Pinning to a
/// build that is absent stops the game launching at all, which is worse than
/// the version Steam would have picked for itself.
///
/// `installed` holds the directory names found in `compatibilitytools.d`, and
/// has to be read after anything that installs one, not before.
#[must_use]
pub fn resolve_wishes(wishes: &[CompatWish], installed: &[String]) -> Vec<CompatTarget> {
    wishes
        .iter()
        .filter_map(|wish| {
            Some(CompatTarget {
                app_id: wish.app_id,
                name: wish.name.clone(),
                tool: tool_for(&wish.choice, installed)?,
                rank: wish.rank,
            })
        })
        .collect()
}

/// The tool name a choice resolves to on this machine.
fn tool_for(choice: &ProtonChoice, installed: &[String]) -> Option<String> {
    match choice {
        ProtonChoice::NewestGeProton => newest_ge_proton(installed).map(str::to_owned),
        ProtonChoice::Experimental => Some(PROTON_EXPERIMENTAL.to_owned()),
        ProtonChoice::Pinned { tool } => Some(tool.clone()),
    }
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
        let path = compat_section(target.app_id);
        let borrowed: Vec<&str> = path.iter().map(String::as_str).collect();

        // Steam writes all three keys for every entry it owns. Writing only the
        // name leaves an entry Steam re-renders with the other two on its next
        // exit, which reads as gameready's change being partly undone.
        let values = [
            (COMPAT_NAME_KEY, target.tool.as_str()),
            (COMPAT_CONFIG_KEY, ""),
            (COMPAT_PRIORITY_KEY, target.rank.priority()),
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

/// Where one game's compatibility entry sits in the config.
#[must_use]
pub fn compat_section(app_id: AppId) -> Vec<String> {
    let mut path: Vec<String> = COMPAT_MAPPING_PATH
        .iter()
        .map(|part| (*part).to_owned())
        .collect();
    path.push(app_id.to_string());
    path
}

/// What every target's entry held before the run wrote into it.
///
/// All three keys, because the run writes all three: putting back only the name
/// would leave the priority gameready raised in place.
pub fn capture_compat_targets(
    text: &str,
    targets: &[(CompatTarget, String)],
) -> Result<Vec<PriorSection>, VdfError> {
    targets
        .iter()
        .map(|(target, _)| {
            let section = compat_section(target.app_id);
            let borrowed: Vec<&str> = section.iter().map(String::as_str).collect();
            let prior = capture_block(
                text,
                &borrowed,
                &[COMPAT_NAME_KEY, COMPAT_CONFIG_KEY, COMPAT_PRIORITY_KEY],
            )?;
            Ok(PriorSection { section, prior })
        })
        .collect()
}

#[cfg(test)]
#[path = "compat_test.rs"]
mod compat_test;
