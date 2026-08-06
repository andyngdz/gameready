//! Write per-game launch options into Steam's own config.

use std::path::PathBuf;

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{Change, digest};
use crate::steps::constants::LOCAL_CONFIG_BACKUP;
use crate::steps::domain::{Edited, LaunchTarget, apply_targets};

/// Sets the launch options of the selected games.
///
/// Built with the games the user picked rather than discovered at probe time,
/// so it is not part of `core_steps()`; `init` constructs it and runs it as its
/// own step once the picker has answered.
///
/// The whole file is backed up before it is touched. It holds every game's
/// playtime and cloud sync state as well as launch options, and a rollback that
/// restored only the one value would be a rollback that could not put a
/// mistake right.
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
        runner
            .read_to_string(&self.config)
            .map_err(|source| StepError::Read {
                path: self.config.clone(),
                source: std::io::Error::other(source.to_string()),
            })
    }
}

impl Improvement for SteamLaunchOptions {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Set Steam launch options for the selected games"
    }

    fn rationale(&self) -> &str {
        "Launch options are how a wrapper such as gamemode or mangohud gets in \
         front of a game, and Steam has no other way to set them per game. The \
         whole config file is copied first, so undoing this puts back exactly \
         what Steam had, including anything you had typed in the box yourself."
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

        // The pre-image goes down before the journal record, so the record never
        // names a backup that is not on disk yet. Written owner-only: this file
        // carries an encrypted app ticket and a cloud key, and a copy of them is
        // kept for every run in a directory nothing prunes.
        let backup = cx.backup_dir().join(LOCAL_CONFIG_BACKUP);
        cx.reader()
            .write_private_file(&backup, &original)
            .map_err(|source| StepError::Write {
                path: backup.clone(),
                source: std::io::Error::other(source.to_string()),
            })?;

        let config = self.config.clone();
        let text = edited.text;
        cx.mutate(
            Change::FileWritten {
                path: config.clone(),
                existed: true,
                backup: Some(backup),
                sha256_after: digest(&text),
                mode: 0o644,
                // The file is in the user's home. A rollback that restored it
                // as root would leave it owned by root, and Steam would then
                // fail to save its own settings.
                privilege: Privilege::User,
            },
            |runner| {
                runner
                    .write_file(&config, &text, Privilege::User)
                    .map_err(|source| StepError::Write {
                        path: config.clone(),
                        source: std::io::Error::other(source.to_string()),
                    })
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
                    "not set".to_owned()
                } else {
                    target.options.clone()
                },
            ));
        }
        Ok(verification)
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        for change in undo.iter().rev() {
            match change {
                Change::FileWritten {
                    path,
                    backup: Some(backup),
                    ..
                } => {
                    let original =
                        cx.reader()
                            .read_to_string(backup)
                            .map_err(|source| StepError::Read {
                                path: backup.clone(),
                                source: std::io::Error::other(source.to_string()),
                            })?;
                    cx.reader()
                        .write_file(path, &original, Privilege::User)
                        .map_err(|source| StepError::Write {
                            path: path.clone(),
                            source: std::io::Error::other(source.to_string()),
                        })?;
                }
                // Listed rather than wildcarded, so a new change this step
                // starts recording fails to compile here instead of being
                // silently skipped by rollback.
                Change::FileWritten { backup: None, .. }
                | Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
                | Change::PackagesInstalled { .. }
                | Change::SystemdUnit { .. }
                | Change::DirCreated { .. }
                | Change::DirTreeInstalled { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "steam_launch_options_test.rs"]
mod steam_launch_options_test;
