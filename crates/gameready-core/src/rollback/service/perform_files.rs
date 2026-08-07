//! Reversing a change that touched a file or a directory.

use std::path::Path;

use crate::exec::{Cmd, CommandRunner};
use crate::improvement::Privilege;
use crate::journal::digest;
use crate::rollback::domain::UndoOutcome;
use crate::steps::MANAGED_HEADER;

/// Deletes a file gameready created, unless its contents no longer match.
///
/// A mismatch has two very different causes and they must not read the same.
/// The file may have been rewritten by a *later* gameready run, since the
/// managed header carries the run id and so two applies of the same step do not
/// produce identical bytes. Or a person edited it. The first is our own work
/// and points at the run to undo instead; the second is theirs and is left
/// alone, because a stale drop-in is recoverable and a clobbered hand edit is
/// not.
pub(super) fn delete_file(
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
pub(super) fn restore_file(
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

/// Recursively removes a directory tree gameready installed.
pub(super) fn remove_dir_tree(
    runner: &dyn CommandRunner,
    path: &Path,
    privilege: Privilege,
) -> UndoOutcome {
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
pub(super) fn remove_dir(runner: &dyn CommandRunner, path: &Path) -> UndoOutcome {
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
#[path = "perform_files_test.rs"]
mod perform_files_test;
