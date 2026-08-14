//! Reversing one recorded change.

use std::path::Path;

use crate::exec::{Cmd, CommandRunner};
use crate::improvement::Privilege;
use crate::journal::{PriorUnitState, Undo};
use crate::rollback::domain::UndoOutcome;
use crate::rollback::service::perform_files::{
    delete_file, remove_dir, remove_dir_tree, restore_file,
};
use crate::steps::SYSCTL_BIN;
use crate::systemd::{DISABLE, NOW, RESTART, SYSTEMCTL};

/// Reverses one recorded change.
pub(super) fn perform(undo: &Undo, runner: &dyn CommandRunner) -> UndoOutcome {
    match undo {
        Undo::DeleteFile {
            path,
            expect_sha256,
            privilege,
        } => delete_file(runner, path, expect_sha256, *privilege),

        Undo::RestoreFile {
            path,
            from,
            privilege,
            ..
        } => restore_file(runner, path, from, *privilege),

        Undo::SetSysctl { key, value } => set_sysctl(runner, key, value),

        Undo::WriteSysfs { path, value } => write_back(runner, path, value),

        Undo::ReportPackages { installed, .. } => report_packages(installed),

        Undo::RestoreUnit { unit, prior } => restore_unit(runner, unit, *prior),

        Undo::RemoveDirIfEmpty { path, privilege } => remove_dir(runner, path, *privilege),

        Undo::RemoveDirTree { path, privilege } => remove_dir_tree(runner, path, *privilege),
    }
}

/// Sets a kernel parameter back to its prior value.
fn set_sysctl(runner: &dyn CommandRunner, key: &str, value: &str) -> UndoOutcome {
    let cmd = Cmd::root(SYSCTL_BIN)
        .arg("-w")
        .arg(format!("{key}={value}"));
    match runner.run(&cmd) {
        Ok(_) => UndoOutcome::Reverted {
            detail: format!("{key} back to {value}"),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Writes a sysfs attribute back to its prior value.
fn write_back(runner: &dyn CommandRunner, path: &Path, value: &str) -> UndoOutcome {
    match runner.write_file(path, value, Privilege::Root) {
        Ok(()) => UndoOutcome::Reverted {
            detail: format!("{} back to {value}", path.display()),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Reports packages rather than removing them.
///
/// Removing a package is not the inverse of installing one: dependency
/// cascades, leftover configuration, and other users of the package all differ
/// from the original operation.
fn report_packages(installed: &[String]) -> UndoOutcome {
    UndoOutcome::Left {
        reason: format!("left installed: {}", installed.join(", ")),
    }
}

/// Returns a unit to its prior state.
///
/// A unit that was enabled before the run is given back running: its drop-in
/// was removed by an earlier undo, so the restart starts whatever scheduler
/// the unit's own config names. A unit the run enabled is disabled again.
fn restore_unit(runner: &dyn CommandRunner, unit: &str, prior: PriorUnitState) -> UndoOutcome {
    match prior {
        PriorUnitState::WasEnabled => {
            let cmd = Cmd::root(SYSTEMCTL).arg(RESTART).arg(unit);
            match runner.run(&cmd) {
                Ok(_) => UndoOutcome::Reverted {
                    detail: format!("{unit} restarted on its own config"),
                },
                Err(error) => UndoOutcome::Failed {
                    error: error.to_string(),
                },
            }
        }
        PriorUnitState::WasDisabled => {
            let cmd = Cmd::root(SYSTEMCTL).arg(DISABLE).arg(NOW).arg(unit);
            match runner.run(&cmd) {
                Ok(_) => UndoOutcome::Reverted {
                    detail: format!("{unit} disabled again"),
                },
                Err(error) => UndoOutcome::Failed {
                    error: error.to_string(),
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "perform_test.rs"]
mod perform_test;
