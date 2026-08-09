//! Install the tools the per-game settings rely on.

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::pkg::PackageManager;
use crate::steps::domain::GAMING_TOOLS;
use crate::steps::use_cases::gaming_tools_survey::{absent, present, ToolSurvey};

/// Puts the three standard gaming tools on the system.
///
/// Neither is tuning in itself. gamemode is what actually moves the governor
/// and the scheduling priority while a game runs, which is why
/// `core.cpu.governor` leaves the governor alone. mangohud is how a user
/// measures whether any of this helped; whether it appears in a launch option
/// is a separate question the run asks, and the answer does not change what
/// gets installed.
#[derive(Debug, Default, Clone, Copy)]
pub struct GamingTools;

impl GamingTools {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.pkg.tools")
    }

    /// The package tooling, or an error naming what is missing.
    ///
    /// A step that installs packages and was handed no package manager cannot
    /// guess; saying so is the only honest answer.
    fn packages<'a>(cx: &CoreCx<'a>) -> Result<&'a dyn PackageManager, StepError> {
        cx.packages.ok_or_else(|| StepError::PreconditionLost {
            step: Self::id_const(),
            detail: "no package tooling was available for this run".to_owned(),
        })
    }

    /// What this machine is missing and what it would take to get it.
    fn survey(cx: &CoreCx<'_>) -> Result<ToolSurvey, StepError> {
        ToolSurvey::read(cx, Self::packages(cx)?)
    }
}

impl Improvement for GamingTools {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Install gamemode and mangohud"
    }

    fn short_name(&self) -> &str {
        "gamemode + mangohud"
    }

    fn bar_name(&self) -> &str {
        "Gaming tools"
    }

    fn blurb(&self) -> &str {
        "gamemode and mangohud"
    }

    fn gains(&self) -> Option<&str> {
        Some(
            "gamemode tunes each game while it runs and puts it back after; mangohud \
             shows whether it helped.",
        )
    }

    fn rationale(&self) -> &str {
        "gamemode raises the CPU governor and process priority for the duration \
         of a game and puts them back afterwards, which is safer than pinning \
         the governor system-wide. mangohud is how you see whether any change \
         helped, which is the only honest way to justify the rest of this. Both \
         are ordinary packages and come off the same way they went on."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu, Tag::Overlay]
    }
}

impl CoreImprovement for GamingTools {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if absent(cx).is_empty() {
            // Naming the binaries rather than saying "installed", which reads
            // in the summary as though this run had just installed them.
            return Ok(Probe::AlreadyApplied {
                evidence: format!(
                    "{} already on PATH",
                    GAMING_TOOLS
                        .iter()
                        .map(|tool| tool.binary)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
        }

        let Some(packages) = cx.packages else {
            return Ok(Probe::Unknown {
                reason: "no package tooling was available to check what can be installed"
                    .to_owned(),
            });
        };

        if ToolSurvey::read(cx, packages)?.installable().is_empty() {
            // A package can be missing from a family's repositories entirely,
            // and a user on that family should read that rather than watch the
            // step fail.
            return Ok(Probe::NotApplicable {
                reason: format!(
                    "none of the missing tools are in this system's repositories ({})",
                    cx.facts.distro.name,
                ),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let packages = Self::survey(cx)?.planned();
        let summary = format!(
            "install {}",
            packages
                .iter()
                .map(|package| package.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(
            StepPlan::new(self.id(), summary).action(PlannedAction::InstallPackages {
                packages,
                already_present: present(cx),
            }),
        )
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let packages = Self::packages(&cx.cx)?;
        let names = ToolSurvey::read(&cx.cx, packages)?.installable();

        if names.is_empty() {
            return Err(StepError::PreconditionLost {
                step: self.id(),
                detail: "nothing left to install since the plan was made".to_owned(),
            });
        }

        cx.progress(&format!("Installing {}", names.join(", ")));
        cx.mutate(
            Change::PackagesInstalled {
                manager: packages.kind().binary().to_owned(),
                requested: names.clone(),
                newly_installed: names.clone(),
            },
            |runner| {
                packages.install(runner, &names)?;
                Ok(())
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let mut verification = Verification::new();

        // Only the tools this system can actually get are checked. Claiming a
        // package failed on a family that does not carry it would be blaming
        // the step for the distribution's choice.
        for tool in GAMING_TOOLS.iter().filter(|tool| {
            tool.spec
                .name_for(cx.facts.distro.package_manager())
                .is_some()
        }) {
            let present = cx.runner.which(tool.binary).is_some();
            verification = verification.check(Check::equals(
                format!("{} on PATH", tool.binary),
                "yes",
                if present { "yes" } else { "no" },
            ));
        }
        Ok(verification)
    }

    fn rollback(
        &self,
        undo: &[Change],
        _cx: &mut ApplyCx<'_, CoreCx<'_>>,
    ) -> Result<(), StepError> {
        for change in undo {
            match change {
                // Removing a package is not the inverse of installing one:
                // dependency cascades and other users of the same package make
                // it a different operation with a wider blast radius. Rollback
                // reports and leaves; `--purge-packages` opts into removal.
                Change::PackagesInstalled { .. } => {}
                // Listed rather than wildcarded, so a new change this step
                // starts recording fails to compile here instead of being
                // silently skipped by rollback.
                Change::FileWritten { .. }
                | Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
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
#[path = "gaming_tools_test.rs"]
mod gaming_tools_test;
