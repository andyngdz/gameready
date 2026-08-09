//! What this system has of the scx packages, and what it would take to get them.

use crate::facts::PackageManagerKind;
use std::path::Path;

use crate::improvement::{ApplyCx, CoreCx, PlannedPackage, Probe, StepError};
use crate::journal::Change;
use crate::pkg::{PackageManager, PackageState};
use crate::steps::constants::{LAVD_SCHEDULER, SCXCTL_BIN, SCX_UNIT_PATH};
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

/// Whether the tooling is here or can be fetched.
///
/// Split from the step's `probe` so the kernel question and the packaging
/// question stay separate: a kernel without sched_ext is a permanent no, and
/// missing packages are a no only until they are installed. Lives here because
/// every answer it gives comes from the package survey below.
pub(super) fn probe_tooling(cx: &CoreCx<'_>) -> Result<Probe, StepError> {
    // Either mechanism being present is enough. Ubuntu has the unit and no
    // scxctl; Arch and Fedora have scxctl and no unit.
    if cx.runner.which(SCXCTL_BIN).is_some() || cx.runner.path_exists(Path::new(SCX_UNIT_PATH)) {
        return Ok(Probe::Applicable);
    }

    let Some(packages) = cx.packages else {
        return Ok(Probe::Unknown {
            reason: "no package tooling was available to check whether scx can be installed"
                .to_owned(),
        });
    };

    let survey = ScxPackages::read(cx, packages)?;
    if survey.can_install() {
        return Ok(Probe::Applicable);
    }
    Ok(Probe::NotApplicable {
        reason: survey.why_not(cx),
    })
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
    /// The repository is not there yet when this probe is asked, but the run is
    /// about to add it and will ask again afterwards, so this says what is true
    /// right now without sending the user away to run gameready twice.
    pub fn why_not(&self, cx: &CoreCx<'_>) -> String {
        if cx.facts.distro.package_manager() == PackageManagerKind::Apt {
            return format!(
                "scx is not in this system's repositories yet ({}); the \"{}\" step in this \
                 run adds the PPA that carries it, and this step is looked at again once it has",
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
    ///
    /// Read from `wanted` rather than `fetchable`, because on Ubuntu the PPA
    /// that makes them resolvable is added by another step in this same run.
    /// Listing only what apt can already see would show an empty install
    /// screen and then fetch 178 MB nobody agreed to.
    pub fn to_install(&self) -> Vec<PlannedPackage> {
        self.wanted()
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

    /// What this machine does not have yet, whatever apt can currently see.
    ///
    /// The plan-time list. A package the repositories do not carry yet is still
    /// wanted: another step in this run is about to make it resolvable.
    fn wanted(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates
            .iter()
            .filter(|candidate| !matches!(candidate.state, PackageState::Installed { .. }))
    }

    /// What the package manager can fetch right now.
    ///
    /// The apply-time list, read again after the repository has been added, so
    /// it is narrower than `wanted` only when something went wrong.
    fn fetchable(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.state.needs_install())
    }
}

#[cfg(test)]
#[path = "scx_lavd_packages_test.rs"]
mod scx_lavd_packages_test;
