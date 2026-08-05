//! `gameready rollback`.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::journal::{self, Journal, RunId, StatePaths};

use crate::cli::ui;
use gameready_core::rollback::{PackagePolicy, RollbackError, execute, latest_run, plan};
use gameready_core::run::RunStatus;

/// Reverses a previous run's changes, newest change first.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    requested: Option<&str>,
    packages: PackagePolicy,
) -> Result<(RunStatus, String)> {
    let records = journal::load(&paths.journal()).context("could not read the journal")?;

    let target = match requested {
        Some(text) => RunId::parse(text).ok_or(RollbackError::MalformedRun {
            requested: text.to_owned(),
        })?,
        None => latest_run(&records).ok_or(RollbackError::NothingRecorded)?,
    };

    let undo_plan = plan(&records, target)?;
    if undo_plan.is_empty() {
        return Ok((
            RunStatus::Clean,
            format!("\nRun {target} recorded no changes to undo.\n"),
        ));
    }

    // Journalled under a new run id, so a rollback that itself fails partway is
    // inspectable rather than invisible.
    let mut journal = Journal::open(paths.clone(), RunId::generate())?;
    let report = execute(&undo_plan, runner, &mut journal, packages)?;

    let status = if report.failed() == 0 {
        RunStatus::Clean
    } else {
        RunStatus::StepFailed
    };
    Ok((
        status,
        ui::RollbackSummary::new(&report, &paths.journal()).to_string(),
    ))
}

#[cfg(test)]
#[path = "rollback_test.rs"]
mod rollback_test;
