//! Undoing a run without Steam throwing the restore away.

use crate::exec::CommandRunner;
use crate::infra::steam::process::{is_running, shutdown, start};
use crate::journal::Journal;
use crate::rollback::{execute, PackagePolicy, RollbackError, RollbackPlan, RollbackReport};
use crate::steps::{SteamLaunchOptions, SteamProton};

/// Reverses a run, quitting Steam first when the run changed a file Steam owns.
///
/// Steam keeps `localconfig.vdf` and `config.vdf` in memory and writes both out
/// when it exits, so a pre-image put back underneath a running Steam is thrown
/// away the next time the user closes it, without a word. The write path hits
/// the same wall and gets around it the same way.
///
/// Steam is left as it was found. A run that never touched Steam does not close
/// it, and a Steam that was not running when the rollback started is not opened
/// at the end.
pub fn undo_with_steam_closed(
    plan: &RollbackPlan,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
    packages: PackagePolicy,
) -> Result<RollbackReport, RollbackError> {
    if !touches_steam_config(plan) {
        return execute(plan, runner, journal, packages);
    }

    let was_running = is_running(runner);
    if was_running {
        shutdown(runner)?;
    }

    let report = execute(plan, runner, journal, packages);
    if was_running {
        start(runner);
    }
    report
}

/// Whether the plan puts back a file Steam holds in memory.
///
/// Decided from the step rather than from the path. A step id is the journal key
/// and never changes, while the two config files sit wherever the user installed
/// Steam, and that same directory also holds things Steam only reads at startup,
/// such as an installed Proton build. Closing a running game client to undo one
/// of those would be worse than the bug this avoids.
fn touches_steam_config(plan: &RollbackPlan) -> bool {
    let owners = [SteamLaunchOptions::id_const(), SteamProton::id_const()];
    plan.undos
        .iter()
        .any(|planned| owners.contains(&planned.step))
}

#[cfg(test)]
#[path = "undo_settings_test.rs"]
mod undo_settings_test;
