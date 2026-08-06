//! `gameready apply`.

use anyhow::{Context as _, Result};
use gameready_core::facts;

use crate::cli::commands::constants::{CANNOT_OPEN_JOURNAL, CANNOT_READ_SYSTEM};
use crate::cli::commands::selection::select_steps;
use gameready_core::exec::CommandRunner;
use gameready_core::infra::pkg;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{Mode, RunReport, execute};

use crate::cli::ui;

/// Probes, applies, and verifies the selected steps.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    step: Option<&str>,
    mode: Mode,
) -> Result<(RunReport, String)> {
    let selected = select_steps(step)?;

    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let pm = pkg::for_kind(facts.distro.package_manager());
    let mut journal =
        Journal::open(paths.clone(), RunId::generate()).context(CANNOT_OPEN_JOURNAL)?;

    let report = execute(
        selected,
        &facts,
        runner,
        &mut journal,
        mode,
        Some(pm.as_ref()),
        &mut |_| {},
    )
    .context("the run could not complete")?;

    let rendered = ui::Summary::new(&report, &paths.journal()).to_string();
    Ok((report, rendered))
}

#[cfg(test)]
#[path = "apply_test.rs"]
mod apply_test;
