//! `gameready doctor`.

use anyhow::{Context as _, Result};
use gameready_core::doctor;
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::ui::layout::{Mark, Section};

/// Reports system facts and what each step currently finds.
pub fn run(runner: &dyn CommandRunner) -> Result<String> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    // The package tooling is what lets a step answer "is this in your
    // repositories" rather than "I could not tell", which is the difference
    // between a useful doctor line and a shrug.
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    let mut out = String::new();
    let mut report = Section::new(&mut out);

    report.blank()?;
    report.title("System")?;
    report.labelled("distro", &facts.distro.name)?;
    report.labelled("family", &facts.distro.family.to_string())?;
    report.labelled("packages", &facts.distro.package_manager().to_string())?;
    report.labelled("kernel", &facts.kernel_release)?;

    report.blank()?;
    report.title(crate::cli::ui::STEPS)?;
    for step in core_steps() {
        let state = step.probe(&cx).map_or_else(
            |error| format!("probe failed: {}", error.describe()),
            |probe| probe.describe(),
        );
        report.row(Mark::None, step.id().as_str(), Some(&state))?;
    }

    let warnings = doctor::check_warnings(&facts, runner);
    if !warnings.is_empty() {
        report.blank()?;
        report.title("Warnings")?;
        for warning in &warnings {
            report.marked(Mark::Warning, &warning.finding)?;
            report.sub(&warning.explanation)?;
            report.sub(&format!("Fix: {}", warning.suggestion))?;
        }
    }

    Ok(out)
}

#[cfg(test)]
#[path = "doctor_test.rs"]
mod doctor_test;
