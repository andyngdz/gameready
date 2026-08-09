//! `gameready doctor`.

use anyhow::{Context as _, Result};
use gameready_core::doctor::{self, machine_report, StepFinding};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::ui::DoctorReport;

/// Reports what the machine is and what each tuning would do, changing nothing.
pub fn run(runner: &dyn CommandRunner) -> Result<String> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    // The package tooling is what lets a step answer "is this in your
    // repositories" rather than "I could not tell".
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());
    let machine = machine_report(runner);

    let findings: Vec<StepFinding> = core_steps()
        .iter()
        .map(|step| StepFinding::of(step.as_ref(), &cx))
        .collect();
    let warnings = doctor::check_warnings(&facts, runner);

    Ok(DoctorReport::new(&facts, &machine, &findings, &warnings).to_string())
}

#[cfg(test)]
#[path = "doctor_test.rs"]
mod doctor_test;
