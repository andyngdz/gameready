//! Working out which Proton build each selected game should run.

use crate::steam::GameSetup;
use crate::steps::{CompatRank, CompatWish};

/// What the selected games ask for, in their profiles' own terms.
///
/// A game whose profile says nothing about Proton is left out rather than
/// pinned to anything. Steam picks a version for itself, and overwriting that
/// choice for a game nobody said anything about is a change nobody asked for.
///
/// Nothing here reads a disk, so a run can count these while it is still asking
/// questions. Turning a wish into a build name is `resolve_wishes`, and belongs
/// after whatever installs one.
#[must_use]
pub fn compat_wishes_for(selected: &[GameSetup]) -> Vec<CompatWish> {
    selected
        .iter()
        .filter_map(|setup| {
            Some(CompatWish {
                app_id: setup.game.app_id,
                name: setup.game.name.clone(),
                choice: setup.profile.proton.clone()?,
                rank: CompatRank::Game,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "compat_test.rs"]
mod compat_test;
