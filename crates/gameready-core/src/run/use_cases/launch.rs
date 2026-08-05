//! Running the Steam launch-options step for the games a user picked.

use crate::games::AppId;
use crate::steam::GameSetup;
use crate::steps::LaunchTarget;

/// The launch-option targets for the selected games.
///
/// A game whose profile asks for nothing is left out rather than given an empty
/// string: writing an empty value would clear whatever the user had typed in
/// Steam's box themselves, which is a change nobody asked for.
#[must_use]
pub fn targets_for(selected: &[GameSetup]) -> Vec<LaunchTarget> {
    selected
        .iter()
        .filter_map(|setup| {
            let options = setup.launch_options();
            if options.is_empty() {
                return None;
            }
            Some(LaunchTarget {
                app_id: AppId(setup.game.app_id.0),
                name: setup.game.name.clone(),
                options,
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "launch_test.rs"]
mod launch_test;
