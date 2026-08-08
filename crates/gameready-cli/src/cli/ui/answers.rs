//! What the user decided, and the one pass that collects it.

use anyhow::Result;
use gameready_core::run::{compat_targets_for, targets_for, InstallConsent};
use gameready_core::steam::{with_overlay, GameSetup, Overlay};
use gameready_core::steps::{CompatTarget, LaunchTarget};

use crate::cli::ui::{LaunchChoice, Questions, SteamWork, Steps};

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
    /// Whether those games show a frame-rate overlay. Already folded into
    /// `selected`, and kept because the plan screen has to say which way it
    /// went and cannot read it back out of a launch option.
    pub overlay: Overlay,
    /// Whether the CPU governor, if pinned this run, should survive a reboot.
    pub governor_pinned: bool,
}

/// Asks every question the run has, in one pass.
///
/// Nothing here changes the machine, and nothing after here asks. That ordering
/// is the point: a question that arrives once packages are installed and the
/// sysctl is written is a question the user cannot really answer, because the
/// alternative is no longer available.
pub fn ask_everything(questions: &Questions<'_>) -> Result<Answers> {
    let mut steps = Steps::of(questions.count());

    let picked = questions.pick_games(&mut steps)?;
    let overlay = questions.pick_overlay(&picked, &mut steps)?;

    let selected = with_overlay(&picked, overlay);
    let targets = targets_for(&selected);
    let proton = compat_targets_for(&selected, questions.compat_tools);

    let work = SteamWork {
        launch: targets.len(),
        proton: proton.len(),
    };
    let launch = questions.pick_launch(&work, &mut steps)?;
    let consent = questions.consent(&mut steps)?;
    let governor_pinned = questions.governor_choice(&consent, &mut steps)?;

    Ok(Answers {
        selected,
        targets,
        proton,
        launch,
        consent,
        overlay,
        governor_pinned,
    })
}

#[cfg(test)]
#[path = "answers_test.rs"]
mod answers_test;
