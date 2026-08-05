//! `gameready apply`.

use anyhow::{Context as _, Result};
use gameready_core::facts;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use gameready_core::improvement::ImprovementId;
use gameready_core::improvement::Privilege;
use gameready_core::infra::exec::RealRunner;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{Mode, RunReport, execute};
use gameready_core::steps::{core_steps, find_core_step};

use crate::cli::ui;

/// Probes, applies, and verifies the selected steps.
pub fn run(
    runner: &RealRunner,
    paths: StatePaths,
    step: Option<&str>,
    mode: Mode,
) -> Result<(RunReport, String)> {
    let selected = match step {
        Some(requested) => {
            let id = ImprovementId::parse(requested)
                .with_context(|| format!("`{requested}` is not a step id"))?;
            vec![find_core_step(&id).with_context(|| format!("no step named `{requested}`"))?]
        }
        None => core_steps(),
    };

    // Prompt once, up front, rather than letting the first privileged command
    // fail against a cold credential cache halfway through a run.
    let needs_root = selected
        .iter()
        .any(|step| matches!(step.privilege(), Privilege::Root));
    if mode.mutates() && needs_root {
        runner
            .prime()
            .context("could not get permission to make system changes")?;
    }

    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let mut journal =
        Journal::open(paths.clone(), RunId::generate()).context("could not open the journal")?;

    let report = execute(selected, &facts, runner, &mut journal, mode, &mut |_| {})
        .context("the run could not complete")?;

    let rendered = ui::render(&report, &paths.journal());
    Ok((report, rendered))
}

#[cfg(test)]
#[path = "apply_test.rs"]
mod apply_test;
