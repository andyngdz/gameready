//! Pin each game to the Proton build its profile asks for.

use std::path::PathBuf;

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::steps::constants::NOT_SET;
use crate::steps::domain::{
    apply_compat_targets, capture_compat_targets, CompatEdited, CompatTarget,
};
use crate::steps::use_cases::restore_steam_config::restore_steam_config;

/// The label every row shows for this step. One constant because the
/// terminal and the panel menu want the same words here.
const SHORT_NAME: &str = "Proton pin";

/// Writes Steam's compatibility tool mapping for the selected games.
///
/// Built from the games the user picked rather than discovered at probe time,
/// so it is not part of `core_steps()`; `init` constructs it and runs it once
/// the picker has answered.
///
/// Only this game's mapping entry is recorded for undo, not a copy of the file.
/// `config.vdf` holds the machine-wide Steam settings too, and Steam rewrites
/// the whole file on exit, so a pre-image restore would undo far more than the
/// run did.
#[derive(Debug, Clone)]
pub struct SteamProton {
    config: PathBuf,
    targets: Vec<CompatTarget>,
}

impl SteamProton {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("game.steam.proton")
    }

    #[must_use]
    pub const fn new(config: PathBuf, targets: Vec<CompatTarget>) -> Self {
        Self { config, targets }
    }

    /// Every target applied to `text`.
    fn edit(&self, text: &str) -> Result<CompatEdited, StepError> {
        apply_compat_targets(text, &self.targets).map_err(StepError::from)
    }

    /// The config file as it stands.
    fn read(&self, runner: &dyn crate::exec::CommandRunner) -> Result<String, StepError> {
        runner.read_to_string(&self.config).map_err(StepError::Exec)
    }
}

impl Improvement for SteamProton {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Pin the Proton version for the selected games"
    }

    fn short_name(&self) -> &str {
        SHORT_NAME
    }

    fn bar_name(&self) -> &str {
        "Proton pin"
    }

    fn blurb(&self) -> &str {
        "A pinned Proton build"
    }

    fn gains(&self) -> Option<&str> {
        Some("Each game runs on the Proton build its profile asks for.")
    }

    fn rationale(&self) -> &str {
        "A game that needs a particular Proton build needs it every time it \
         starts, and Steam's default is whatever it picks for itself. This sets \
         the same thing the Compatibility tab sets, from the profile that says \
         which build the game wants. Whatever the Compatibility tab said first \
         is recorded, so undoing this puts that back and nothing else."
    }

    fn privilege(&self) -> Privilege {
        // The file is in the user's own home. Writing it as root would leave it
        // owned by root, and Steam would then fail to save its own settings.
        Privilege::User
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Steam]
    }
}

impl CoreImprovement for SteamProton {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if self.targets.is_empty() {
            return Ok(Probe::NotApplicable {
                reason: "no selected game asks for a particular Proton version".to_owned(),
            });
        }
        if !cx.runner.path_exists(&self.config) {
            return Ok(Probe::NotApplicable {
                reason: format!("{} does not exist", self.config.display()),
            });
        }

        let text = self.read(cx.runner)?;
        if self.edit(&text)?.replaced.is_empty() {
            return Ok(Probe::AlreadyApplied {
                evidence: "every game already runs the version its profile asks for".to_owned(),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let text = self.read(cx.runner)?;
        let edited = self.edit(&text)?;

        let mut plan = StepPlan::new(
            self.id(),
            format!("pin Proton for {} game(s)", edited.replaced.len()),
        );
        for (target, previous) in &edited.replaced {
            plan = plan.action(PlannedAction::RunCommand {
                display: if previous.is_empty() {
                    format!("{}: Proton -> {}", target.name, target.tool)
                } else {
                    format!("{}: Proton `{previous}` -> `{}`", target.name, target.tool)
                },
            });
        }
        Ok(plan)
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let original = self.read(cx.reader())?;
        let edited = self.edit(&original)?;
        if edited.replaced.is_empty() {
            return Ok(());
        }

        // Read off the file as it stands, before the write, so the undo names
        // values that were really there.
        let sections = capture_compat_targets(&original, &edited.replaced)?;

        let config = self.config.clone();
        let text = edited.text;
        cx.mutate(
            Change::SteamConfigWritten {
                path: config.clone(),
                sections,
            },
            |runner| {
                // Written as the user: the file is in the user's home, and a
                // root-owned copy stops Steam saving its own settings.
                runner
                    .write_file(&config, &text, Privilege::User)
                    .map_err(StepError::Exec)
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let text = self.read(cx.runner)?;
        let edited = self.edit(&text)?;
        let mut verification = Verification::new();

        // Nothing left to change means every target reads back as asked for.
        for target in &self.targets {
            let still_pending = edited.is_pending(target.app_id);
            verification = verification.check(Check::equals(
                format!("{} Proton version", target.name),
                target.tool.clone(),
                if still_pending {
                    NOT_SET.to_owned()
                } else {
                    target.tool.clone()
                },
            ));
        }
        Ok(verification)
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        restore_steam_config(undo, cx)
    }
}

#[cfg(test)]
#[path = "steam_proton_test.rs"]
mod steam_proton_test;
