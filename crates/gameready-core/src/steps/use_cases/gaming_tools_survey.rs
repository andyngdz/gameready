//! Reading what this machine already has of the gaming tools.

use crate::improvement::{CoreCx, PlannedPackage, StepError};
use crate::pkg::{PackageManager, PackageState};
use crate::steps::domain::{GamingTool, GAMING_TOOLS};

/// The tools whose executable is not on `PATH`.
///
/// Probed by looking up the binary rather than by asking the package manager,
/// because a user who built one of these by hand has it and does not need the
/// package.
pub fn absent(cx: &CoreCx<'_>) -> Vec<&'static GamingTool> {
    GAMING_TOOLS
        .iter()
        .filter(|tool| cx.runner.which(tool.binary).is_none())
        .collect()
}

/// The tools this machine already has, by package name.
///
/// The confirmation screen shows these so a step titled "install gamemode and
/// mangohud" that is only fetching one of them does not read as though it lost
/// the other.
pub fn present(cx: &CoreCx<'_>) -> Vec<String> {
    let family = cx.facts.distro.package_manager();
    GAMING_TOOLS
        .iter()
        .filter(|tool| cx.runner.which(tool.binary).is_some())
        .filter_map(|tool| tool.spec.name_for(family))
        .map(str::to_owned)
        .collect()
}

/// One absent tool and what the package manager says about it.
struct Candidate {
    tool: &'static GamingTool,
    package: String,
    state: PackageState,
}

/// What it would take to get the tools this machine is missing.
///
/// Read once and answered from several times, because each `state` call runs a
/// package-manager query and the step asks the same question from `probe`,
/// `plan`, and `apply`.
pub struct ToolSurvey {
    candidates: Vec<Candidate>,
}

impl ToolSurvey {
    /// Asks the package manager about every absent tool.
    ///
    /// A tool with no name on this family is left out entirely: that is known
    /// without asking the system, and querying a name that does not exist would
    /// only produce a confusing error.
    pub fn read(cx: &CoreCx<'_>, packages: &dyn PackageManager) -> Result<Self, StepError> {
        let family = cx.facts.distro.package_manager();
        let mut candidates = Vec::new();

        for tool in absent(cx) {
            let Some(package) = tool.spec.name_for(family) else {
                continue;
            };
            candidates.push(Candidate {
                tool,
                package: package.to_owned(),
                state: packages.state(cx.runner, package)?,
            });
        }
        Ok(Self { candidates })
    }

    /// The names that will actually be installed, in table order.
    pub fn installable(&self) -> Vec<String> {
        self.fetchable()
            .map(|candidate| candidate.package.clone())
            .collect()
    }

    /// The same packages, carrying the text that lets a user agree to them.
    pub fn planned(&self) -> Vec<PlannedPackage> {
        self.fetchable()
            .map(|candidate| PlannedPackage {
                name: candidate.package.clone(),
                what: candidate.tool.what.to_owned(),
                why: candidate.tool.why.to_owned(),
                approx_bytes: candidate.tool.spec.approx_bytes,
            })
            .collect()
    }

    fn fetchable(&self) -> impl Iterator<Item = &Candidate> {
        self.candidates
            .iter()
            .filter(|candidate| candidate.state.needs_install())
    }
}

#[cfg(test)]
#[path = "gaming_tools_survey_test.rs"]
mod gaming_tools_survey_test;
