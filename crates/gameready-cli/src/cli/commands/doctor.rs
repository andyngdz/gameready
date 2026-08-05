//! `gameready doctor`.

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use gameready_core::improvement::{CoreCx, Probe};
use gameready_core::steps::core_steps;

/// Reports system facts and what each step currently finds.
pub fn run(runner: &dyn CommandRunner) -> Result<String> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let cx = CoreCx::new(&facts, runner);

    let mut out = String::new();
    let _ = writeln!(out, "\nSystem");
    let _ = writeln!(out, "  kernel   {}", facts.kernel_release);

    let _ = writeln!(out, "\nSteps");
    for step in core_steps() {
        let state = match step.probe(&cx) {
            Ok(Probe::Applicable) => "would apply".to_owned(),
            Ok(Probe::AlreadyApplied { evidence }) => format!("already set ({evidence})"),
            Ok(Probe::NotApplicable { reason }) => format!("not applicable ({reason})"),
            Ok(Probe::Conflict { with, .. }) => format!("conflicts with {with}"),
            Ok(Probe::Unknown { reason }) => format!("could not tell ({reason})"),
            Err(error) => format!("probe failed: {error}"),
        };
        let _ = writeln!(out, "  {}  {state}", step.id());
    }

    Ok(out)
}

#[cfg(test)]
#[path = "doctor_test.rs"]
mod doctor_test;
