//! `gameready rollback`.

use anyhow::Result;
use gameready_core::exec::CommandRunner;
use gameready_core::journal::{Journal, RunId, StatePaths};

use crate::cli::escalation::Escalation;
use crate::cli::ui;
use crate::features::rollback_plan;
use gameready_core::infra::steam::{is_running, undo_with_steam_closed};
use gameready_core::run::RunStatus;

/// Reverses a previous run's changes, newest change first.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    requested: Option<&str>,
    escalation: Escalation<'_>,
) -> Result<(RunStatus, String)> {
    let undo_plan = rollback_plan(&paths, requested)?;
    if undo_plan.is_empty() {
        return Ok((
            RunStatus::Clean,
            format!("\nRun {} recorded no changes to undo.\n", undo_plan.run),
        ));
    }

    if console::user_attended_stderr() {
        eprint!("{}", ui::preview(&undo_plan, runner));
    }

    if undo_plan.touches_steam() && is_running(runner) && !ui::confirm_steam_close()? {
        return Ok((
            RunStatus::Clean,
            [
                "",
                "Nothing undone. Close Steam yourself, then run `gameready rollback` again.",
                "",
            ]
            .join("\n"),
        ));
    }

    if undo_plan.needs_root() {
        escalation.ask()?;
    }

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
