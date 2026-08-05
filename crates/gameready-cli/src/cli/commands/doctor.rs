//! `gameready doctor`.

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;

/// Reports system facts and what each step currently finds.
pub fn run(runner: &dyn CommandRunner) -> Result<String> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let cx = CoreCx::new(&facts, runner);

    // `?` rather than a discarded Result: writing to a String cannot fail, but
    // the formatting machinery still returns one, and `fmt::Error` converts
    // into the anyhow error this already returns.
    let mut out = String::new();
    writeln!(out, "\nSystem")?;
    writeln!(out, "  distro    {}", facts.distro.name)?;
    writeln!(out, "  family    {}", facts.distro.family)?;
    writeln!(out, "  packages  {}", facts.distro.package_manager())?;
    writeln!(out, "  kernel    {}", facts.kernel_release)?;

    writeln!(out, "\nSteps")?;
    for step in core_steps() {
        let state = step.probe(&cx).map_or_else(
            |error| format!("probe failed: {error}"),
            |probe| probe.describe(),
        );
        writeln!(out, "  {}  {state}", step.id())?;
    }

    Ok(out)
}

#[cfg(test)]
#[path = "doctor_test.rs"]
mod doctor_test;
