//! Let gamemode actually raise a game's priority, which its default does not.

use std::path::PathBuf;

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change};
use crate::steps::constants::{GAMEMODE_INI, MANAGED_HEADER, NOT_SET};
use crate::steps::domain::GAMEMODE;
use crate::steps::use_cases::gamemode_config_file::{self as file, RENICE};
use crate::steps::use_cases::gamemode_config_group::{
    in_gamemode_group, GamemodeGroup, JOIN_GROUP,
};
use crate::steps::use_cases::gaming_tools::GamingTools;
use crate::steps::use_cases::user_home::user_home;

/// The step whose install turns this one from "nothing to configure" into a
/// real choice.
///
/// A `static` rather than a `const` for the reason given on the same list in
/// `cpu_governor`: a const is inlined at every use site, so `requires` would
/// hand back a reference to a temporary.
static UNLOCKED_BY: [ImprovementId; 1] = [GamingTools::id_const()];

/// Configures gamemode to renice the games it manages.
#[derive(Debug, Clone)]
pub struct GamemodeConfig {
    config: PathBuf,
}

impl GamemodeConfig {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.gamemode.config")
    }

    /// Resolves the config path from the invoking user's home.
    #[must_use]
    pub fn detect() -> Self {
        Self {
            config: user_home().join(GAMEMODE_INI),
        }
    }

    /// Uses an explicit path, for tests that control it.
    #[cfg(test)]
    #[must_use]
    pub fn with_config(config: PathBuf) -> Self {
        Self { config }
    }

    /// The file's current body, empty when it is not there yet.
    fn current_body(&self, cx: &CoreCx<'_>) -> Result<String, StepError> {
        if !cx.runner.path_exists(&self.config) {
            return Ok(String::new());
        }
        cx.runner
            .read_to_string(&self.config)
            .map_err(StepError::Exec)
    }
}

impl Improvement for GamemodeConfig {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Let gamemode raise the priority of a running game"
    }

    fn short_name(&self) -> &str {
        "gamemode.ini"
    }

    fn bar_name(&self) -> &str {
        "GameMode config"
    }

    fn blurb(&self) -> &str {
        "gamemode priority"
    }

    fn gains(&self) -> Option<&str> {
        Some("A running game keeps the CPU when something else on the desktop wants it.")
    }

    fn undo_note(&self) -> Option<&str> {
        Some("no reboot; gamemode rereads the file")
    }

    fn rationale(&self) -> &str {
        "gamemode can give a running game a higher scheduling priority, but its \
         shipped default for that is 0, which means it does nothing. Installing \
         gamemode and leaving the file alone buys the governor change and no \
         priority change. This writes the one setting that turns it on. It \
         needs your user to be in the gamemode group, so the step stands down \
         rather than writing a file that would be ignored."
    }

    fn privilege(&self) -> Privilege {
        Privilege::User
    }

    /// gamemode arriving mid-run turns "not installed yet" into a real choice.
    fn requires(&self) -> &[ImprovementId] {
        &UNLOCKED_BY
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu]
    }
}

impl CoreImprovement for GamemodeConfig {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if cx.runner.which(GAMEMODE.binary).is_none() {
            return Ok(Probe::NotApplicable {
                reason: "gamemode is not installed yet".to_owned(),
            });
        }

        // A file gameready did not write is the user's own, and this step
        // creates rather than edits, so there is nothing safe to do to it.
        let existing = self.current_body(cx)?;
        if !existing.is_empty() {
            if !existing.contains(MANAGED_HEADER) {
                return Ok(Probe::Conflict {
                    with: "your own gamemode.ini".to_owned(),
                    detail: format!("{} is yours, not gameready's", self.config.display()),
                    yours: None,
                });
            }
            if file::sets_renice(&existing) {
                return Ok(Probe::AlreadyApplied {
                    evidence: format!("{} already sets renice", self.config.display()),
                });
            }
        }

        // gamemoded reads the calling process's groups, so a user added to the
        // group but not yet logged back in still would not get the renice.
        // Writing the file anyway would leave a setting that silently does
        // nothing, which is worse than not writing it.
        if in_gamemode_group(cx.runner)? == GamemodeGroup::Absent {
            return Ok(Probe::NotApplicable {
                reason: format!("your user is not in the gamemode group, run `{JOIN_GROUP}`"),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, _cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        Ok(
            StepPlan::new(self.id(), format!("gamemode renice {RENICE}")).action(
                PlannedAction::CreateFile {
                    path: self.config.to_string_lossy().into_owned(),
                    contents: file::preview(),
                },
            ),
        )
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let config = self.config.clone();
        let contents = file::contents(self.id(), cx.run());
        let sha256_after = digest(&contents);

        cx.mutate(
            Change::FileWritten {
                path: config.clone(),
                existed: false,
                backup: None,
                sha256_after,
                mode: 0o644,
                privilege: Privilege::User,
            },
            |runner| {
                runner
                    .write_file(&config, &contents, Privilege::User)
                    .map_err(StepError::Exec)
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let existing = self.current_body(cx)?;

        Ok(Verification::new()
            .check(Check::equals(
                format!("{} exists", self.config.display()),
                "yes",
                if existing.is_empty() { "no" } else { "yes" },
            ))
            .check(Check::equals(
                "gamemode renice",
                RENICE.to_string(),
                if file::sets_renice(&existing) {
                    RENICE.to_string()
                } else {
                    NOT_SET.to_owned()
                },
            )))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        for change in undo.iter().rev() {
            match change {
                Change::FileWritten { path, .. } => {
                    cx.reader()
                        .remove_file(path, Privilege::User)
                        .map_err(StepError::Exec)?;
                }
                // Listed rather than wildcarded: a new Change variant this
                // step starts recording must fail to compile here rather than
                // be silently skipped by rollback.
                Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
                | Change::PackagesInstalled { .. }
                | Change::SystemdUnit { .. }
                | Change::AptRepository { .. }
                | Change::ScxScheduler { .. }
                | Change::DirCreated { .. }
                | Change::DirTreeInstalled { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "gamemode_config_test.rs"]
mod gamemode_config_test;
