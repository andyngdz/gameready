//! Install gamemode, mangohud, and gamescope.

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::pkg::{PackageManager, PackageState};
use crate::steps::domain::{GAMING_TOOLS, GamingTool};

/// Puts the three standard gaming tools on the system.
///
/// These are not tuning in themselves. gamemode is what actually moves the
/// governor and the scheduling priority while a game runs, which is why
/// `core.cpu.governor` leaves the governor alone; mangohud is how a user
/// measures whether any of this helped; gamescope fixes a class of windowing
/// problems no sysctl can.
#[derive(Debug, Default, Clone, Copy)]
pub struct GamingTools;

/// One absent tool and what the package manager says about it.
struct Candidate {
    package: String,
    state: PackageState,
}

impl GamingTools {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.pkg.tools")
    }

    /// The tools whose executable is not on `PATH`.
    ///
    /// Probed by looking up the binary rather than by asking the package
    /// manager, because a user who built gamescope by hand has it and does not
    /// need the package.
    fn absent(cx: &CoreCx<'_>) -> Vec<&'static GamingTool> {
        GAMING_TOOLS
            .iter()
            .filter(|tool| cx.runner.which(tool.binary).is_none())
            .collect()
    }

    /// What the package manager says about each absent tool.
    ///
    /// A tool with no name on this family is left out entirely: that is known
    /// without asking the system, and querying a name that does not exist would
    /// only produce a confusing error.
    fn candidates(
        cx: &CoreCx<'_>,
        packages: &dyn PackageManager,
    ) -> Result<Vec<Candidate>, StepError> {
        let family = cx.facts.distro.package_manager();
        let mut candidates = Vec::new();

        for tool in Self::absent(cx) {
            let Some(package) = tool.spec.name_for(family) else {
                continue;
            };
            candidates.push(Candidate {
                package: package.to_owned(),
                state: packages.state(cx.runner, package)?,
            });
        }
        Ok(candidates)
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

    /// The names that will actually be installed, in table order.
    fn installable(candidates: &[Candidate]) -> Vec<String> {
        candidates
            .iter()
            .filter(|candidate| candidate.state.needs_install())
            .map(|candidate| candidate.package.clone())
            .collect()
    }
}

impl Improvement for GamingTools {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Install gamemode, mangohud, and gamescope"
    }

    fn rationale(&self) -> &str {
        "gamemode raises the CPU governor and process priority for the duration \
         of a game and puts them back afterwards, which is safer than pinning \
         the governor system-wide. mangohud is how you see whether any change \
         helped. gamescope gives a game its own compositor, which fixes alt-tab \
         and resolution handling. All three are ordinary packages and come off \
         the same way they went on."
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
        if Self::absent(cx).is_empty() {
            return Ok(Probe::AlreadyApplied {
                evidence: "gamemoded, mangohud, and gamescope are all on PATH".to_owned(),
            });
        }

        let Some(packages) = cx.packages else {
            return Ok(Probe::Unknown {
                reason: "no package tooling was available to check what can be installed"
                    .to_owned(),
            });
        };

        let candidates = Self::candidates(cx, packages)?;
        if Self::installable(&candidates).is_empty() {
            // Debian 12 has no gamescope at all, and a user on it should read
            // that rather than watch the step fail.
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
        let candidates = Self::candidates(cx, Self::packages(cx)?)?;
        let names = Self::installable(&candidates);

        Ok(
            StepPlan::new(self.id(), format!("install {}", names.join(", ")))
                .action(PlannedAction::InstallPackages { names }),
        )
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let packages = Self::packages(&cx.cx)?;
        let candidates = Self::candidates(&cx.cx, packages)?;
        let names = Self::installable(&candidates);

        if names.is_empty() {
            return Err(StepError::PreconditionLost {
                step: self.id(),
                detail: "nothing left to install since the plan was made".to_owned(),
            });
        }

        // Which names are new is decided before the install, not read back from
        // it. That is what lets the undo record be durable first: a transaction
        // interrupted halfway still has every name it could have added on disk.
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

        // Only the tools this system can actually get are checked. Claiming
        // gamescope on Debian 12 failed would be blaming the step for a package
        // that does not exist there.
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
                | Change::DirCreated { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "gaming_tools_test.rs"]
mod gaming_tools_test;
