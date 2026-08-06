//! Reversing one recorded change.

use std::path::Path;

use crate::exec::{Cmd, CommandRunner};
use crate::improvement::Privilege;
use crate::journal::{PriorUnitState, Undo, digest};
use crate::rollback::domain::{PackagePolicy, UndoOutcome};
use crate::steps::{MANAGED_HEADER, SYSCTL_BIN};

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

        Undo::RemoveDirIfEmpty { path } => remove_dir(runner, path),

        Undo::RemoveDirTree { path, privilege } => remove_dir_tree(runner, path, *privilege),
    }
}

/// Deletes a file gameready created, unless its contents no longer match.
///
/// A mismatch has two very different causes and they must not read the same.
/// The file may have been rewritten by a *later* gameready run, since the
/// managed header carries the run id and so two applies of the same step do not
/// produce identical bytes. Or a person edited it. The first is our own work
/// and points at the run to undo instead; the second is theirs and is left
/// alone, because a stale drop-in is recoverable and a clobbered hand edit is
/// not.
fn delete_file(
    runner: &dyn CommandRunner,
    path: &Path,
    expect_sha256: &str,
    privilege: Privilege,
) -> UndoOutcome {
    if !runner.path_exists(path) {
        return UndoOutcome::AlreadyGone;
    }

    let current = match runner.read_to_string(path) {
        Ok(current) => current,
        Err(error) => {
            return UndoOutcome::Failed {
                error: error.to_string(),
            };
        }
    };

    if digest(&current) != expect_sha256 {
        return mismatch(path, &current);
    }

    match runner.remove_file(path, privilege) {
        Ok(()) => UndoOutcome::Reverted {
            detail: format!("removed {}", path.display()),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Explains a file whose contents are not what this run wrote.
fn mismatch(path: &Path, current: &str) -> UndoOutcome {
    let Some(owner) = managed_run(current) else {
        return UndoOutcome::Refused {
            reason: format!(
                "{} was edited after gameready wrote it, so it was left alone",
                path.display()
            ),
        };
    };

    UndoOutcome::Left {
        reason: format!(
            "{} now belongs to run {owner}; undo that one with \
             `gameready rollback --run {owner}`",
            path.display()
        ),
    }
}

/// Reads the run id out of a gameready-managed file's header.
///
/// `None` means the file is not ours, so nothing may be assumed about it.
fn managed_run(contents: &str) -> Option<&str> {
    contents
        .lines()
        .find(|line| line.starts_with(MANAGED_HEADER))?
        .split("run=")
        .nth(1)
        .map(str::trim)
}

/// Puts a pre-image back.
fn restore_file(
    runner: &dyn CommandRunner,
    path: &Path,
    from: &Path,
    privilege: Privilege,
) -> UndoOutcome {
    let contents = match runner.read_to_string(from) {
        Ok(contents) => contents,
        Err(error) => {
            return UndoOutcome::Failed {
                error: error.to_string(),
            };
        }
    };

    match runner.write_file(path, &contents, privilege) {
        Ok(()) => UndoOutcome::Reverted {
            detail: format!("restored {}", path.display()),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
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

    let cmd = Cmd::root("systemctl").arg("disable").arg("--now").arg(unit);
    match runner.run(&cmd) {
        Ok(_) => UndoOutcome::Reverted {
            detail: format!("{unit} disabled again"),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Recursively removes a directory tree gameready installed.
fn remove_dir_tree(runner: &dyn CommandRunner, path: &Path, privilege: Privilege) -> UndoOutcome {
    if !runner.path_exists(path) {
        return UndoOutcome::AlreadyGone;
    }
    let cmd = match privilege {
        Privilege::Root => Cmd::root("rm"),
        Privilege::User => Cmd::user("rm"),
    }
    .arg("-rf")
    .arg(path.to_string_lossy().into_owned());
    match runner.run(&cmd) {
        Ok(_) => UndoOutcome::Reverted {
            detail: format!("removed {}", path.display()),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Removes a directory gameready created, unless something else uses it.
fn remove_dir(runner: &dyn CommandRunner, path: &Path) -> UndoOutcome {
    match runner.remove_file(path, Privilege::Root) {
        Ok(()) => UndoOutcome::Reverted {
            detail: format!("removed {}", path.display()),
        },
        Err(_) => UndoOutcome::Left {
            reason: format!("{} was not empty, so it was left alone", path.display()),
        },
    }
}

#[cfg(test)]
#[path = "perform_test.rs"]
mod perform_test;
