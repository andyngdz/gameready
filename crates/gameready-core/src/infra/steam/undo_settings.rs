//! Undoing a run without Steam throwing the restore away.

use crate::exec::CommandRunner;
use crate::infra::steam::process::{is_running, shutdown, start};
use crate::journal::Journal;
use crate::rollback::{execute, RollbackError, RollbackPlan, RollbackReport};

/// Reverses a run, quitting Steam first when the run changed a file Steam owns.
///
/// Steam keeps `localconfig.vdf` and `config.vdf` in memory and writes both out
/// when it exits, so a pre-image put back underneath a running Steam is thrown
/// away the next time the user closes it, without a word. The write path hits
/// the same wall and gets around it the same way.
///
/// Steam is left as it was found. A run that never touched Steam does not close
/// it, and a Steam that was not running when the rollback started is not opened
/// at the end. Whether closing Steam is allowed is the caller's decision: the
/// CLI asks the user before this is reached, so a rollback never closes a
/// running game client on its own.
pub fn undo_with_steam_closed(
    plan: &RollbackPlan,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
) -> Result<RollbackReport, RollbackError> {
    if !plan.touches_steam() {
        return execute(plan, runner, journal);
    }

    let was_running = is_running(runner);
    if was_running {
        shutdown(runner)?;
    }

    let report = execute(plan, runner, journal);
    if was_running {
        start(runner);
    }
    report
}

#[cfg(test)]
#[path = "undo_settings_test.rs"]
mod undo_settings_test;
