//! Everything the run needs to ask, asked before anything is changed.

use anyhow::Result;
use gameready_core::facts::PackageManagerKind;
use gameready_core::run::{compat_targets_for, targets_for, InstallConsent, Mode, RunPlan};
use gameready_core::steam::{with_overlay, GameSetup, Overlay};
use gameready_core::steps::{CompatTarget, LaunchTarget};

use crate::cli::ui::{
    choose_games, choose_how_to_apply, choose_overlay, consent_to_install, LaunchChoice, SteamWork,
};

/// Whether there is a person at the terminal to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Picker {
    /// Ask. The normal path.
    Ask,
    /// Take every installed game, say yes to the prerequisites, and answer
    /// nothing, for a script or a terminal that cannot prompt.
    TakeAll,
}

/// Everything the run already worked out, ready to be put to the user.
pub struct Questions<'a> {
    /// The games found on this machine.
    pub setups: &'a [GameSetup],
    /// What the steps would do and what they need first.
    pub plan: &'a RunPlan,
    /// This machine's package tooling, for naming packages the way it does.
    pub packages: PackageManagerKind,
    /// Whether anyone is there to answer.
    pub picker: Picker,
    /// `Some` only when a flag already settled the overlay.
    pub overlay: Option<Overlay>,
    /// Whether this run may change anything.
    pub mode: Mode,
    /// The compatibility tools installed on this machine, by directory name.
    ///
    /// A profile asking for the newest GE-Proton resolves against this, so a
    /// machine with none installed pins nothing rather than pinning a game to
    /// a build that is not there.
    pub compat_tools: &'a [String],
}

impl Questions<'_> {
    /// Whether the run may install what its steps need.
    fn consent(&self) -> Result<InstallConsent> {
        consent_to_install(self.plan, self.packages, self.picker, self.mode)
    }
}

/// What the user decided, before a single change was made.
pub struct Answers {
    /// The games to set up, with the overlay answer already folded in.
    pub selected: Vec<GameSetup>,
    /// The launch options to write, empty when there are none.
    pub targets: Vec<LaunchTarget>,
    /// The Proton pins to write, empty when no profile asks for one.
    pub proton: Vec<CompatTarget>,
    /// How both of those should be applied.
    pub launch: LaunchChoice,
    /// Whether the run may install the packages its steps need.
    pub consent: InstallConsent,
}

/// Asks every question the run has, in one pass.
///
/// Nothing here changes the machine, and nothing after here asks. That ordering
/// is the point: a question that arrives once packages are installed and the
/// sysctl is written is a question the user cannot really answer, because the
/// alternative is no longer available.
pub fn ask_everything(questions: &Questions<'_>) -> Result<Answers> {
    let picked = match questions.picker {
        Picker::Ask => choose_games(questions.setups)?,
        // Every game, not only the ones a profile matched: a run that cannot
        // ask has no way to learn that the user wanted the rest, and every game
        // now has settings to write.
        Picker::TakeAll => questions.setups.to_vec(),
    };

    let overlay = match (questions.overlay, questions.picker) {
        (Some(chosen), _) => chosen,
        (None, Picker::Ask) if !picked.is_empty() => choose_overlay()?,
        (None, Picker::Ask | Picker::TakeAll) => Overlay::default_answer(),
    };

    let selected = with_overlay(&picked, overlay);
    let targets = targets_for(&selected);
    let proton = compat_targets_for(&selected, questions.compat_tools);

    let work = SteamWork {
        launch: targets.len(),
        proton: proton.len(),
    };
    let launch = if targets.is_empty() && proton.is_empty() {
        LaunchChoice::ShowForCopying
    } else {
        match (questions.picker, questions.mode.mutates()) {
            (Picker::Ask, true) => choose_how_to_apply(&work)?,
            (Picker::TakeAll, _) | (Picker::Ask, false) => LaunchChoice::ShowForCopying,
        }
    };

    Ok(Answers {
        selected,
        targets,
        proton,
        launch,
        consent: questions.consent()?,
    })
}

#[cfg(test)]
#[path = "questions_test.rs"]
mod questions_test;
