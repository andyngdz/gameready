//! What this system has of the scx packages, and what it would take to get them.

use crate::facts::PackageManagerKind;
use crate::improvement::{ApplyCx, CoreCx, PlannedPackage, StepError};
use crate::journal::Change;
use crate::pkg::{PackageManager, PackageState};
use crate::steps::constants::LAVD_SCHEDULER;
use crate::steps::domain::{SCX_SCHEDS, SCX_TOOLS};
use crate::steps::use_cases::scx_ppa::ScxPpa;

/// What a package is for, said once so the plan screen and the survey agree.
const SCHEDS_WHAT: &str = "the sched_ext CPU schedulers, scx_lavd among them";
const TOOLS_WHAT: &str = "the loader and the scxctl command that switches schedulers";

/// One scx package and what the package manager says about it.
struct Candidate {
    name: String,
    what: &'static str,
    approx_bytes: u64,
    state: PackageState,
}

/// Both scx packages, resolved for this distro.
///
/// Ubuntu ships everything in one package, so this can legitimately hold one
/// entry rather than two. What matters is whether every entry it does hold can
/// be installed, not how many there are.
pub struct ScxPackages {
    candidates: Vec<Candidate>,
}

impl ScxPackages {
    /// Asks the package manager about every scx package this family names.
    pub fn read(cx: &CoreCx<'_>, packages: &dyn PackageManager) -> Result<Self, StepError> {
        let family = cx.facts.distro.package_manager();
        let mut candidates = Vec::new();

        for (spec, what) in [(SCX_SCHEDS, SCHEDS_WHAT), (SCX_TOOLS, TOOLS_WHAT)] {
            let Some(name) = spec.name_for(family) else {
                continue;
            };
            candidates.push(Candidate {
                name: name.to_owned(),
                what,
                approx_bytes: spec.approx_bytes,
                state: packages.state(cx.runner, name)?,
            });
        }
        Ok(Self { candidates })
    }

    /// Whether every package this family needs is installed or installable.
    ///
    /// A family that names no package at all cannot install anything, which is
    /// why an empty list is a no rather than a vacuous yes.
    pub fn can_install(&self) -> bool {
        !self.candidates.is_empty()
            && self
                .candidates
                .iter()
                .all(|candidate| candidate.state != PackageState::Unavailable)
    }

    /// Why this system cannot get scx, in terms a user can act on.
    ///
    /// Apt gets a different answer because gameready has a step that fixes it.
    /// Every step is probed before any step applies, so the repository this run
    /// is about to add is not there yet when this one is asked: the honest
    /// answer names the second run rather than pretending one will do.
    pub fn why_not(&self, cx: &CoreCx<'_>) -> String {
        if cx.facts.distro.package_manager() == PackageManagerKind::Apt {
            return format!(
                "scx is not in this system's repositories yet ({}); the \"{}\" step in this \
                 run adds the PPA that carries it, and this step applies the next time you \
                 run gameready",
                cx.facts.distro.name,
                ScxPpa::id_const(),
            );
        }
        format!(
            "scx is not in this system's repositories ({}); on Fedora it comes from the \
             CachyOS COPR, which gameready does not add for you",
            cx.facts.distro.name,
        )
    }

    /// The packages that will be fetched, with the text the screen asks with.
    pub fn to_install(&self) -> Vec<PlannedPackage> {
        self.fetchable()
            .map(|candidate| PlannedPackage {
                name: candidate.name.clone(),
                what: candidate.what.to_owned(),
                why: format!("{LAVD_SCHEDULER} cannot be loaded without it"),
                approx_bytes: candidate.approx_bytes,
            })
            .collect()
    }

    /// The packages this machine already has.
    pub fn already_here(&self) -> Vec<String> {
        self.candidates
            .iter()
            .filter(|candidate| matches!(candidate.state, PackageState::Installed { .. }))
            .map(|candidate| candidate.name.clone())
            .collect()
    }

    /// Installs whatever is missing, recording only what was genuinely new.
    ///
    /// A no-op when everything is already here, which is the normal case on a
    /// machine that has run this step before or installed scx by hand.
    pub fn install_missing(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let Some(packages) = cx.cx.packages else {
            return Ok(());
        };
        let survey = Self::read(&cx.cx, packages)?;
        let names: Vec<String> = survey
            .fetchable()
            .map(|candidate| candidate.name.clone())
            .collect();
        if names.is_empty() {
            return Ok(());
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

    fn fetchable(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.state.needs_install())
    }
}

#[cfg(test)]
#[path = "scx_lavd_packages_test.rs"]
mod scx_lavd_packages_test;
