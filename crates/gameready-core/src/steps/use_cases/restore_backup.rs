//! Putting back a file a step overwrote, from the copy it took first.

use crate::improvement::{ApplyCx, CoreCx, Privilege, StepError};
use crate::journal::Change;

/// Restores every file in `undo` from its backup, newest change first.
///
/// The step-owned half of undoing a write. `gameready rollback` has its own
/// engine for a finished run; this is the path a step takes when its own
/// verification failed a moment after it wrote.
///
/// Written back as the user rather than as root, because both files this covers
/// live in the user's home and Steam stops being able to save its own settings
/// once root owns them.
pub fn restore_from_backup(
    undo: &[Change],
    cx: &mut ApplyCx<'_, CoreCx<'_>>,
) -> Result<(), StepError> {
    for change in undo.iter().rev() {
        match change {
            Change::FileWritten {
                path,
                backup: Some(backup),
                ..
            } => {
                let original =
                    cx.reader()
                        .read_to_string(backup)
                        .map_err(|source| StepError::Read {
                            path: backup.clone(),
                            source: std::io::Error::other(source.to_string()),
                        })?;
                cx.reader()
                    .write_file(path, &original, Privilege::User)
                    .map_err(|source| StepError::Write {
                        path: path.clone(),
                        source: std::io::Error::other(source.to_string()),
                    })?;
            }
            // Listed rather than wildcarded, so a change a caller starts
            // recording fails to compile here instead of being silently
            // skipped by rollback.
            Change::FileWritten { backup: None, .. }
            | Change::FileRemoved { .. }
            | Change::SysctlRuntime { .. }
            | Change::SysfsWrite { .. }
            | Change::PackagesInstalled { .. }
            | Change::SystemdUnit { .. }
            | Change::AptRepository { .. }
            | Change::ScxScheduler { .. }
            | Change::DirCreated { .. }
            | Change::DirTreeInstalled { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "restore_backup_test.rs"]
mod restore_backup_test;
