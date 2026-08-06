//! Everything the run needs to ask, asked before anything is changed.

use anyhow::Result;
use gameready_core::run::{Mode, targets_for};
use gameready_core::steam::{GameSetup, Overlay, with_overlay};
use gameready_core::steps::LaunchTarget;

use crate::cli::ui::{LaunchChoice, choose_games, choose_how_to_apply, choose_overlay};

/// Whether there is a person at the terminal to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    /// Ask. The normal path.
    Ask,
    /// Take every installed game and answer nothing, for a script or a terminal
    /// that cannot prompt.
    TakeAll,
}

/// What the user decided, before a single change was made.
pub struct Answers {
    /// The games to set up, with the overlay answer already folded in.
    pub selected: Vec<GameSetup>,
    /// The launch options to write, empty when there are none.
    pub targets: Vec<LaunchTarget>,
    /// How those launch options should be applied.
    pub launch: LaunchChoice,
}

/// Asks every question the run has, in one pass.
///
/// Nothing here changes the machine, and nothing after here asks. That ordering
/// is the point: a question that arrives once packages are installed and the
/// sysctl is written is a question the user cannot really answer, because the
/// alternative is no longer available.
pub fn ask_everything(
    setups: &[GameSetup],
    picker: Picker,
    overlay: Option<Overlay>,
    mode: Mode,
) -> Result<Answers> {
    let picked = match picker {
        Picker::Ask => choose_games(setups)?,
        // Every game, not only the ones a profile matched: a run that cannot
        // ask has no way to learn that the user wanted the rest, and every game
        // now has settings to write.
        Picker::TakeAll => setups.to_vec(),
    };

    let overlay = match (overlay, picker) {
        (Some(chosen), _) => chosen,
        (None, Picker::Ask) if !picked.is_empty() => choose_overlay()?,
        (None, Picker::Ask | Picker::TakeAll) => Overlay::default_answer(),
    };

    let selected = with_overlay(&picked, overlay);
    let targets = targets_for(&selected);

    let launch = if targets.is_empty() {
        LaunchChoice::ShowForCopying
    } else {
        match (picker, mode.mutates()) {
            (Picker::Ask, true) => choose_how_to_apply(targets.len())?,
            (Picker::TakeAll, _) | (Picker::Ask, false) => LaunchChoice::ShowForCopying,
        }
    };

    Ok(Answers {
        selected,
        targets,
        launch,
    })
}

#[cfg(test)]
#[path = "questions_test.rs"]
mod questions_test;
