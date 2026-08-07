//! Make scx installable on Ubuntu by adding the PPA that carries it.

use std::path::{Path, PathBuf};

use crate::exec::Cmd;
use crate::facts::PackageManagerKind;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{Change, digest};
use crate::pkg::PackageState;
use crate::steps::constants::{
    ADD_APT_REPOSITORY_BIN, APT_ASSUME_YES, APT_REMOVE, SCX_PPA, SCX_PPA_PIN,
};
use crate::steps::domain::SCX_SCHEDS;
use crate::steps::use_cases::scx_ppa_pin;

/// Adds `ppa:arighi/sched-ext`, pinned so it can only supply scx.
///
/// Its own step rather than something the scheduler step does quietly, because
/// a third-party repository outlives the run that added it and can supply any
/// package on the system. That is a decision a user makes once, on a screen
/// that says so.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScxPpa;

impl ScxPpa {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.repo.scx-ppa")
    }

    /// The apt name for scx on this family, when there is one.
    fn package(cx: &CoreCx<'_>) -> Option<&'static str> {
        SCX_SCHEDS.name_for(cx.facts.distro.package_manager())
    }

    /// The command that adds the repository and fetches its signing key.
    fn add() -> Cmd {
        Cmd::root(ADD_APT_REPOSITORY_BIN)
            .arg(APT_ASSUME_YES)
            .arg(SCX_PPA)
    }
}

impl Improvement for ScxPpa {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Add the scx package repository"
    }

    fn rationale(&self) -> &str {
        "Ubuntu's own archive carries no sched_ext schedulers at any release, \
         so scx_lavd cannot be installed without a third-party repository. This \
         adds ppa:arighi/sched-ext, maintained by the same person who maintains \
         scx upstream, and pins it so apt will take the scx package from it and \
         refuse everything else. Without that pin a repository can replace any \
         package on the system, including system ones. Rollback removes both \
         the repository and the pin."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu]
    }
}

impl CoreImprovement for ScxPpa {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if cx.facts.distro.package_manager() != PackageManagerKind::Apt {
            return Ok(Probe::NotApplicable {
                reason: "only apt systems need this; Arch has scx in extra and Fedora has it \
                         in the CachyOS COPR"
                    .to_owned(),
            });
        }

        let (Some(package), Some(packages)) = (Self::package(cx), cx.packages) else {
            return Ok(Probe::Unknown {
                reason: "no package tooling was available to check whether scx already resolves"
                    .to_owned(),
            });
        };

        // The goal is that scx resolves, not that our own file exists. A user
        // who added the PPA by hand has already got what this step is for.
        if packages.state(cx.runner, package)? != PackageState::Unavailable {
            return Ok(Probe::AlreadyApplied {
                evidence: format!("{package} already resolves from a configured repository"),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let package = Self::package(cx).unwrap_or("scx");
        Ok(StepPlan::new(
            self.id(),
            format!("add {SCX_PPA}, pinned to {package} only"),
        )
        .action(PlannedAction::CreateFile {
            path: SCX_PPA_PIN.to_owned(),
            contents: scx_ppa_pin::body(package),
        })
        .action(PlannedAction::RunCommand {
            display: Self::add().to_string(),
        }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let package = Self::package(&cx.cx).unwrap_or("scx");
        let pin = PathBuf::from(SCX_PPA_PIN);
        let contents = scx_ppa_pin::file(package, cx.run());

        // The pin goes down first. Adding the repository before it would leave
        // a window where apt could take any package from the PPA, and a run
        // interrupted inside that window would leave it open for good.
        cx.mutate(
            Change::FileWritten {
                path: pin.clone(),
                existed: false,
                backup: None,
                sha256_after: digest(&contents),
                mode: 0o644,
                privilege: Privilege::Root,
            },
            |runner| {
                runner
                    .write_file(&pin, &contents, Privilege::Root)
                    .map_err(|source| StepError::Write {
                        path: pin.clone(),
                        source: std::io::Error::other(source.to_string()),
                    })
            },
        )?;

        cx.progress(&format!("Adding {SCX_PPA}"));
        cx.mutate(
            Change::AptRepository {
                spec: SCX_PPA.to_owned(),
            },
            |runner| {
                let add = Self::add();
                runner
                    .run(&add)
                    .map(|_| ())
                    .map_err(|source| StepError::Command {
                        command: add.to_string(),
                        code: 1,
                        stderr: source.to_string(),
                    })
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let package = Self::package(cx).unwrap_or("scx");
        let resolves = cx.packages.is_some_and(|packages| {
            packages
                .state(cx.runner, package)
                .is_ok_and(|state| state != PackageState::Unavailable)
        });

        Ok(Verification::new()
            .check(Check::equals(
                format!("{package} resolves"),
                "yes",
                if resolves { "yes" } else { "no" },
            ))
            .check(Check::equals(
                "the PPA is pinned",
                "yes",
                if cx.runner.path_exists(Path::new(SCX_PPA_PIN)) {
                    "yes"
                } else {
                    "no"
                },
            )))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // Reverse order: the repository goes before the pin that restrains it,
        // so an interrupted rollback never leaves the PPA configured without
        // its pin.
        for change in undo.iter().rev() {
            match change {
                Change::AptRepository { spec } => {
                    let remove = Cmd::root(ADD_APT_REPOSITORY_BIN)
                        .arg(APT_ASSUME_YES)
                        .arg(APT_REMOVE)
                        .arg(spec);
                    cx.reader()
                        .run(&remove)
                        .map_err(|source| StepError::Command {
                            command: remove.to_string(),
                            code: 1,
                            stderr: source.to_string(),
                        })?;
                }
                Change::FileWritten { path, .. } => {
                    cx.reader()
                        .remove_file(path, Privilege::Root)
                        .map_err(|source| StepError::Write {
                            path: path.clone(),
                            source: std::io::Error::other(source.to_string()),
                        })?;
                }
                // Listed rather than wildcarded, so a new change this step
                // starts recording fails to compile here instead of being
                // silently skipped by rollback.
                Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
                | Change::PackagesInstalled { .. }
                | Change::SystemdUnit { .. }
                | Change::ScxScheduler { .. }
                | Change::DirCreated { .. }
                | Change::DirTreeInstalled { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "scx_ppa_test.rs"]
mod scx_ppa_test;
