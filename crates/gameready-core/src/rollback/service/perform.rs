//! Reversing one recorded change.

use std::path::Path;

use crate::exec::{Cmd, CommandRunner};
use crate::improvement::Privilege;
use crate::journal::{PriorUnitState, Undo};
use crate::rollback::domain::{PackagePolicy, UndoOutcome};
use crate::rollback::service::perform_files::{
    delete_file, remove_dir, remove_dir_tree, restore_file,
};
use crate::steps::{
    restore_scheduler as restore_scheduler_cmd, ADD_APT_REPOSITORY_BIN, APT_ASSUME_YES, APT_REMOVE,
    SYSCTL_BIN,
};
use crate::systemd::{DISABLE, NOW, SYSTEMCTL};

/// Reverses one recorded change.
pub(super) fn perform(
    undo: &Undo,
    runner: &dyn CommandRunner,
    packages: PackagePolicy,
) -> UndoOutcome {
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

        Undo::ReportPackages { installed, .. } => report_packages(installed, packages),

        Undo::RestoreUnit { unit, prior } => restore_unit(runner, unit, *prior),

        Undo::RemoveAptRepository { spec } => remove_repository(runner, spec),

        Undo::RestoreScxScheduler { previous } => restore_scheduler(runner, previous.as_deref()),

        Undo::RemoveDirIfEmpty { path } => remove_dir(runner, path),

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
fn report_packages(installed: &[String], packages: PackagePolicy) -> UndoOutcome {
    match packages {
        PackagePolicy::Keep => UndoOutcome::Left {
            reason: format!("left installed: {}", installed.join(", ")),
        },
        PackagePolicy::Purge => UndoOutcome::Left {
            reason: "package removal is not implemented yet".to_owned(),
        },
    }
}

/// Returns a unit to its prior state, which usually means disabling it again.
fn restore_unit(runner: &dyn CommandRunner, unit: &str, prior: PriorUnitState) -> UndoOutcome {
    if matches!(prior, PriorUnitState::WasEnabled) {
        return UndoOutcome::Left {
            reason: format!("{unit} was already enabled before the run"),
        };
    }

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

/// Takes a third-party repository back off the system.
///
/// The same tool that added it removes it, so nothing here has to know which
/// files were written or where the signing key landed.
fn remove_repository(runner: &dyn CommandRunner, spec: &str) -> UndoOutcome {
    let cmd = Cmd::root(ADD_APT_REPOSITORY_BIN)
        .arg(APT_ASSUME_YES)
        .arg(APT_REMOVE)
        .arg(spec);
    match runner.run(&cmd) {
        Ok(_) => UndoOutcome::Reverted {
            detail: format!("{spec} removed"),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Hands scheduling back to whatever was running before.
///
/// Unloading takes effect on the next scheduling decision, so this is one of
/// the few undos the user can feel finish. It is also why the step that loads a
/// scheduler never writes a config file to make it persist: there would then be
/// two things to undo, and only one of them instant.
fn restore_scheduler(runner: &dyn CommandRunner, previous: Option<&str>) -> UndoOutcome {
    match runner.run(&restore_scheduler_cmd(previous)) {
        Ok(_) => UndoOutcome::Reverted {
            detail: previous.map_or_else(
                || "the kernel's own scheduler is back".to_owned(),
                |scheduler| format!("{scheduler} is back"),
            ),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

#[cfg(test)]
#[path = "perform_test.rs"]
mod perform_test;
