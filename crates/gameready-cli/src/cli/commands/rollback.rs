//! `gameready rollback`.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::journal::{self, Journal, RunId, StatePaths};

use crate::cli::escalation::Escalation;
use crate::cli::ui;
use gameready_core::infra::steam::{is_running, undo_with_steam_closed};
use gameready_core::rollback::{latest_run, plan, RollbackError};
use gameready_core::run::RunStatus;

/// Reverses a previous run's changes, newest change first.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    requested: Option<&str>,
    escalation: Escalation<'_>,
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

    // Shown before anything changes, so the user is deciding, not reviewing.
    // Read back from the plan and the current machine, which is why the runner
    // is needed here. Printed before the Steam question and the password
    // prompt, each of which is its own chance to back out.
    if console::user_attended_stderr() {
        eprint!("{}", ui::preview(&undo_plan, runner));
    }

    // Undoing a run that wrote into Steam's config means closing Steam, and
    // closing a running game client is the user's call, not the tool's: Steam
    // may be mid-download or running a game. Asked before the password prompt,
    // so a No never triggers one, and only when Steam is actually up.
    if undo_plan.touches_steam() && is_running(runner) && !ui::confirm_steam_close()? {
        return Ok((
            RunStatus::Clean,
            "\nNothing undone. Close Steam yourself, then run \
             `gameready rollback` again.\n"
                .to_owned(),
        ));
    }

    // Asked here rather than at the top, so a malformed run id and a run with
    // nothing to undo both answer without a password. A run that only touched
    // the user's own files is not asked for one at all.
    if undo_plan.needs_root() {
        escalation.ask()?;
    }

    // Journalled under a new run id, so a rollback that itself fails partway is
    // inspectable rather than invisible.
    let mut journal = Journal::open(paths.clone(), RunId::generate())?;
    let report = undo_with_steam_closed(&undo_plan, runner, &mut journal)?;

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
