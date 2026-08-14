//! Write per-game launch options into Steam's own config.

use std::path::PathBuf;

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::steps::constants::NOT_SET;
use crate::steps::domain::{apply_targets, capture_targets, Edited, LaunchTarget};
use crate::steps::use_cases::restore_steam_config::restore_steam_config;

/// The label every row shows for this step. One constant because the
/// terminal and the panel menu want the same words here.
const SHORT_NAME: &str = "Launch options";

/// Sets the launch options of the selected games.
///
/// Built with the games the user picked rather than discovered at probe time,
/// so it is not part of `core_steps()`; `init` constructs it and runs it as its
/// own step once the picker has answered.
///
/// Only the launch options key is recorded for undo, not a copy of the file.
/// Steam owns this file and rewrites it every time it exits, so a rollback that
/// put a whole pre-image back would undo the run and throw away every setting
/// the user changed in Steam since, from playtime to cloud sync state.
#[derive(Debug, Clone)]
pub struct SteamLaunchOptions {
    config: PathBuf,
    targets: Vec<LaunchTarget>,
}

impl SteamLaunchOptions {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("game.steam.launch-options")
    }

    #[must_use]
    pub const fn new(config: PathBuf, targets: Vec<LaunchTarget>) -> Self {
        Self { config, targets }
    }

    /// Every target applied to `text`.
    fn edit(&self, text: &str) -> Result<Edited, StepError> {
        apply_targets(text, &self.targets).map_err(StepError::from)
    }

    /// The config file as it stands.
    fn read(&self, runner: &dyn crate::exec::CommandRunner) -> Result<String, StepError> {
        runner.read_to_string(&self.config).map_err(StepError::Exec)
    }
}

impl Improvement for SteamLaunchOptions {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Set Steam launch options for the selected games"
    }

    fn short_name(&self) -> &str {
        SHORT_NAME
    }

    fn bar_name(&self) -> &str {
        "Launch options"
    }

    fn blurb(&self) -> &str {
        "Steam launch options"
    }

    fn gains(&self) -> Option<&str> {
        Some("Each game launches through the wrappers you chose.")
    }

    fn rationale(&self) -> &str {
        "Launch options are how a wrapper such as gamemode or mangohud gets in \
         front of a game, and Steam has no other way to set them per game. \
         Whatever was in the box first is recorded, so undoing this puts your \
         own launch options back and leaves the rest of Steam's config alone."
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

impl CoreImprovement for SteamLaunchOptions {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if self.targets.is_empty() {
            return Ok(Probe::NotApplicable {
                reason: "no game with launch options was selected".to_owned(),
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
                evidence: "launch options match".to_owned(),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let text = self.read(cx.runner)?;
        let edited = self.edit(&text)?;

        let mut plan = StepPlan::new(
            self.id(),
            format!("set launch options for {} game(s)", edited.replaced.len()),
        );
        for (target, previous) in &edited.replaced {
            plan = plan.action(PlannedAction::RunCommand {
                display: if previous.is_empty() {
                    format!("{}: launch options -> {}", target.name, target.options)
                } else {
                    format!(
                        "{}: launch options `{previous}` -> `{}`",
                        target.name, target.options
                    )
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
        let sections = capture_targets(&original, &edited.replaced)?;

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
                format!("{} launch options", target.name),
                target.options.clone(),
                if still_pending {
                    NOT_SET.to_owned()
                } else {
                    target.options.clone()
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
#[path = "steam_launch_options_test.rs"]
mod steam_launch_options_test;
