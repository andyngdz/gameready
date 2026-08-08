//! Install the latest Proton-GE into Steam's compatibility tools directory.

#[path = "proton_ge_fetch.rs"]
mod proton_ge_fetch;

use std::path::{Path, PathBuf};

use crate::exec::Cmd;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::steps::constants::{COMPAT_TOOLS_DIR, COMPAT_TOOL_VDF, CURL_BIN, TAR_BIN};
use crate::steps::domain::tarball_name;

use proton_ge_fetch::{download_verified, fetch_release};

/// Installs the latest Proton-GE release into Steam's compatibility tools
/// directory so it appears in the per-game Proton version picker.
#[derive(Debug, Clone)]
pub struct ProtonGe {
    compat_dir: PathBuf,
}

impl ProtonGe {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.proton.ge")
    }

    /// Discovers the compat dir from the user's Steam root.
    #[must_use]
    pub fn detect() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_owned());
        Self {
            compat_dir: PathBuf::from(home)
                .join(".steam/root")
                .join(COMPAT_TOOLS_DIR),
        }
    }

    /// Uses an explicit compat dir, for tests that control the path.
    #[cfg(test)]
    #[must_use]
    pub fn with_compat_dir(compat_dir: PathBuf) -> Self {
        Self { compat_dir }
    }

    fn install_dir(&self, tag: &str) -> PathBuf {
        self.compat_dir.join(tag)
    }
}

impl Improvement for ProtonGe {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Install latest Proton-GE"
    }

    fn short_name(&self) -> &str {
        "Proton-GE"
    }

    fn rationale(&self) -> &str {
        "Proton-GE includes game-specific patches and media codecs that Valve's \
         Proton does not ship. Installing it makes it available in Steam's \
         per-game compatibility tool picker on the next restart."
    }

    fn privilege(&self) -> Privilege {
        Privilege::User
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Steam]
    }
}

impl CoreImprovement for ProtonGe {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if cx.runner.which(CURL_BIN).is_none() {
            return Ok(Probe::NotApplicable {
                reason: "curl is not installed".to_owned(),
            });
        }
        let steam_root = self.compat_dir.parent().unwrap_or(Path::new("/"));
        if !cx.runner.path_exists(steam_root) {
            return Ok(Probe::NotApplicable {
                reason: "Steam root not found".to_owned(),
            });
        }
        let release = match fetch_release(cx.runner) {
            Ok(release) => release,
            Err(_) => {
                return Ok(Probe::NotApplicable {
                    reason: "could not reach GitHub API for Proton-GE".to_owned(),
                });
            }
        };
        if cx.runner.path_exists(&self.install_dir(&release.tag)) {
            return Ok(Probe::AlreadyApplied {
                evidence: format!("{} is already installed", release.tag),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let release = fetch_release(cx.runner)?;
        Ok(StepPlan::new(self.id(), format!("install {}", release.tag))
            .action(PlannedAction::RunCommand {
                display: format!("download and verify {}", tarball_name(&release.tag)),
            })
            .action(PlannedAction::RunCommand {
                display: format!("extract to {}", self.install_dir(&release.tag).display()),
            }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        cx.progress("Fetching latest release info");
        let release = fetch_release(cx.reader())?;

        cx.progress(&format!("Downloading {}", tarball_name(&release.tag)));
        let temp_path = download_verified(cx.reader(), &release)?;
        let temp_str = temp_path.to_string_lossy().into_owned();
        let install_dir = self.install_dir(&release.tag);

        cx.progress(&format!("Extracting {} to Steam", release.tag));
        cx.mutate(
            Change::DirTreeInstalled {
                path: install_dir.clone(),
                privilege: Privilege::User,
            },
            |runner| {
                if !runner.path_exists(&self.compat_dir) {
                    let mkdir = Cmd::user("mkdir")
                        .arg("-p")
                        .arg(self.compat_dir.to_string_lossy().into_owned());
                    runner.run(&mkdir).map_err(StepError::Exec)?;
                }
                let extract = Cmd::user(TAR_BIN)
                    .arg("xzf")
                    .arg(&temp_str)
                    .arg("-C")
                    .arg(self.compat_dir.to_string_lossy().into_owned());
                runner.run(&extract).map_err(StepError::Exec)?;
                let _ = runner.remove_file(&temp_path, Privilege::User);
                Ok(())
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let release = fetch_release(cx.runner)?;
        let vdf = self.install_dir(&release.tag).join(COMPAT_TOOL_VDF);

        Ok(Verification::new().check(Check::equals(
            format!("{}/{COMPAT_TOOL_VDF} exists", release.tag),
            "yes",
            if cx.runner.path_exists(&vdf) {
                "yes"
            } else {
                "no"
            },
        )))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        for change in undo.iter().rev() {
            match change {
                Change::DirTreeInstalled { path, privilege } => {
                    let rm = match privilege {
                        Privilege::Root => Cmd::root("rm"),
                        Privilege::User => Cmd::user("rm"),
                    }
                    .arg("-rf")
                    .arg(path.to_string_lossy().into_owned());
                    cx.reader().run(&rm).map_err(StepError::Exec)?;
                }
                Change::FileWritten { .. }
                | Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
                | Change::PackagesInstalled { .. }
                | Change::SystemdUnit { .. }
                | Change::AptRepository { .. }
                | Change::ScxScheduler { .. }
                | Change::DirCreated { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "proton_ge_step_test.rs"]
mod proton_ge_step_test;
