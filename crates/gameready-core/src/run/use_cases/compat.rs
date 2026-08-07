//! Working out which Proton build each selected game should run.

use crate::games::ProtonChoice;
use crate::steam::GameSetup;
use crate::steps::{CompatTarget, PROTON_EXPERIMENTAL, newest_ge_proton};

/// The Proton pins for the selected games.
///
/// A game whose profile says nothing about Proton is left out rather than
/// pinned to anything. Steam picks a version for itself, and overwriting that
/// choice for a game nobody said anything about is a change nobody asked for.
///
/// `installed` holds the tool directory names found in `compatibilitytools.d`,
/// passed in because finding them means reading a disk.
#[must_use]
pub fn compat_targets_for(selected: &[GameSetup], installed: &[String]) -> Vec<CompatTarget> {
    selected
        .iter()
        .filter_map(|setup| {
            let tool = tool_for(setup.profile.proton.as_ref()?, installed)?;
            Some(CompatTarget {
                app_id: setup.game.app_id,
                name: setup.game.name.clone(),
                tool,
            })
        })
        .collect()
}

/// The tool name a choice resolves to on this machine.
///
/// `None` when the profile asks for the newest GE-Proton and none is installed.
/// Nothing is written in that case: pinning a game to a build that is not there
/// stops the game launching at all, which is worse than the default it had.
fn tool_for(choice: &ProtonChoice, installed: &[String]) -> Option<String> {
    match choice {
        ProtonChoice::NewestGeProton => newest_ge_proton(installed).map(str::to_owned),
        ProtonChoice::Experimental => Some(PROTON_EXPERIMENTAL.to_owned()),
        ProtonChoice::Pinned { tool } => Some(tool.clone()),
    }
}

#[cfg(test)]
#[path = "compat_test.rs"]
mod compat_test;
