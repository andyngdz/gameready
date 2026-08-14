//! Everything the run needs to ask, asked before anything is changed.

use anyhow::Result;
use gameready_core::facts::PackageManagerKind;
use gameready_core::improvement::ImprovementId;
use gameready_core::run::{InstallConsent, Mode, RunPlan};
use gameready_core::steam::{GameSetup, Overlay};
use gameready_core::steps::{CompatWish, CpuGovernor, GamingTools};

use crate::cli::ui::{
    choose_games, choose_governor_persistence, choose_how_to_apply, choose_overlay,
    choose_proton_pin, choose_takeover, consent_to_install, LaunchChoice, ProtonPin, SteamWork,
    Steps,
};

/// What the header over the packages question warns about, since it is the one
/// answer in the run that a rollback cannot take back.
const CANNOT_UNDO: &str = "the one thing rollback can't undo";

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
}

impl Questions<'_> {
    /// Which games this run sets up.
    pub(super) fn pick_games(&self, steps: &mut Steps) -> Result<Vec<GameSetup>> {
        match self.picker {
            Picker::Ask if !self.setups.is_empty() => {
                steps.heading(None);
                choose_games(self.setups)
            }
            Picker::Ask => Ok(Vec::new()),
            // Every game, not only the ones a profile matched: a run that
            // cannot ask has no way to learn that the user wanted the rest, and
            // every game now has settings to write.
            Picker::TakeAll => Ok(self.setups.to_vec()),
        }
    }

    /// Whether those games show a frame-rate overlay.
    pub(super) fn pick_overlay(&self, picked: &[GameSetup], steps: &mut Steps) -> Result<Overlay> {
        match (self.overlay, self.picker) {
            (Some(chosen), _) => Ok(chosen),
            (None, Picker::Ask) if !picked.is_empty() => {
                steps.heading(None);
                choose_overlay()
            }
            (None, Picker::Ask | Picker::TakeAll) => Ok(Overlay::default_answer()),
        }
    }

    /// Whether the newest Proton-GE becomes what these games run on.
    ///
    /// Asked before anything is installed, and answered into wishes rather than
    /// build names. Which build is newest is only settled once the run has
    /// finished installing one.
    pub(super) fn pick_proton(
        &self,
        wishes: Vec<CompatWish>,
        steps: &mut Steps,
    ) -> Result<Vec<CompatWish>> {
        if wishes.is_empty() {
            return Ok(wishes);
        }
        match (self.picker, self.mode.mutates()) {
            (Picker::Ask, true) => {
                steps.heading(None);
                Ok(choose_proton_pin(wishes.len())?.wishes(wishes))
            }
            // A dry run previews the fuller answer rather than the emptier one,
            // for the same reason it is the run that lists the packages: it has
            // only this screen to say what a real run would have done.
            (Picker::TakeAll, _) | (Picker::Ask, false) => Ok(ProtonPin::UseNewest.wishes(wishes)),
        }
    }

    /// How the settings Steam holds get applied, when there are any.
    pub(super) fn pick_launch(&self, work: &SteamWork, steps: &mut Steps) -> Result<LaunchChoice> {
        if work.launch == 0 && work.proton == 0 {
            return Ok(LaunchChoice::ShowForCopying);
        }
        match (self.picker, self.mode.mutates()) {
            (Picker::Ask, true) => {
                steps.heading(None);
                choose_how_to_apply(work)
            }
            (Picker::TakeAll, _) | (Picker::Ask, false) => Ok(LaunchChoice::ShowForCopying),
        }
    }

    /// Whether the run may install what its steps need.
    pub(super) fn consent(&self, steps: &mut Steps) -> Result<InstallConsent> {
        if self.asks_about_packages() {
            steps.heading(Some(CANNOT_UNDO));
        }
        consent_to_install(self.plan, self.packages, self.picker, self.mode)
    }

    /// Whether there is anything to install and anyone to ask about it.
    fn asks_about_packages(&self) -> bool {
        matches!(self.picker, Picker::Ask)
            && self.mode.mutates()
            && !self.plan.installs(self.packages).is_empty()
    }

    /// How many questions this run could put, counted before the first answer.
    ///
    /// Every count here is an upper bound: most of these questions only exist
    /// depending on how an earlier one was answered, and there is no way to
    /// know that before asking. Bounding it high is the safe direction. A run
    /// that stops at "3 of 4" is a user who picked nothing at step 1, which
    /// reads as the flow ending early rather than as a number that lied.
    pub(super) fn count(&self) -> usize {
        if matches!(self.picker, Picker::TakeAll) {
            return 0;
        }
        let has_games = !self.setups.is_empty();
        let asked = [
            has_games,
            has_games && self.overlay.is_none(),
            has_games && self.mode.mutates(),
            has_games && self.mode.mutates(),
            self.asks_about_packages(),
            self.mode.mutates() && self.pins_governor(),
        ];
        let mut count = asked.into_iter().filter(|&asked| asked).count();
        if self.mode.mutates() {
            count += self.plan.contested.len();
        }
        count
    }

    /// Whether the governor step is in this run at all.
    fn pins_governor(&self) -> bool {
        self.plan
            .pending
            .iter()
            .any(|step| step.id() == CpuGovernor::id_const())
    }

    /// Whether to put the governor-persistence question to the user.
    ///
    /// Only when the run would actually pin the governor: the step is pending,
    /// and gamemode is not arriving to make it stand down. Asking otherwise
    /// offers a choice with no effect, since the pending-side re-probe would
    /// drop the step the moment gamemode lands.
    fn asks_about_governor(&self, consent: &InstallConsent) -> bool {
        let is_pending = |id| self.plan.pending.iter().any(|step| step.id() == id);
        let gamemode_arriving =
            matches!(consent, InstallConsent::Granted) && is_pending(GamingTools::id_const());
        is_pending(CpuGovernor::id_const()) && !gamemode_arriving
    }

    /// The governor-persistence answer, asked only when it would take effect.
    ///
    /// A run that cannot ask, or one that will not pin the governor, keeps the
    /// safe default: the change lasts this boot only.
    pub(super) fn governor_choice(
        &self,
        consent: &InstallConsent,
        steps: &mut Steps,
    ) -> Result<bool> {
        match self.picker {
            Picker::Ask if self.mode.mutates() && self.asks_about_governor(consent) => {
                steps.heading(None);
                choose_governor_persistence()
            }
            Picker::Ask | Picker::TakeAll => Ok(false),
        }
    }

    /// Which contested steps the user agreed to take over.
    ///
    /// One question per contested step, counting into the run's total like any
    /// other. A run that cannot ask keeps the safe answer: nothing is taken
    /// over that nobody agreed to.
    pub(super) fn takeovers(&self, steps: &mut Steps) -> Result<Vec<ImprovementId>> {
        let mut agreed = Vec::new();
        if !(matches!(self.picker, Picker::Ask) && self.mode.mutates()) {
            return Ok(agreed);
        }
        for contested in &self.plan.contested {
            steps.heading(None);
            if choose_takeover(contested)? {
                agreed.push(contested.step.id());
            }
        }
        Ok(agreed)
    }
}

#[cfg(test)]
#[path = "questions_test.rs"]
mod questions_test;
