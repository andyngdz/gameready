//! Putting back the Steam config keys a step wrote, from what it recorded.

use crate::improvement::{ApplyCx, CoreCx, Privilege, StepError};
use crate::journal::Change;
use crate::steam::restore_sections;

/// Reverses every Steam config write in `undo`, newest change first.
///
/// The step-owned half of undoing a write. `gameready rollback` has its own
/// engine for a finished run; this is the path a step takes when its own
/// verification failed a moment after it wrote.
///
/// Only the recorded keys are put back, never a copy of the whole file. Steam
/// owns `localconfig.vdf` and `config.vdf` and rewrites both on exit, so a
/// pre-image restore would undo the step and take everything else in the file
/// with it.
///
/// Written back as the user rather than as root, because both files live in the
/// user's home and Steam stops being able to save its own settings once root
/// owns them.
pub fn restore_steam_config(
    undo: &[Change],
    cx: &mut ApplyCx<'_, CoreCx<'_>>,
) -> Result<(), StepError> {
    for change in undo.iter().rev() {
        match change {
            Change::SteamConfigWritten { path, sections } => {
                let current = cx.reader().read_to_string(path).map_err(StepError::Exec)?;
                let restored = restore_sections(&current, sections)?;
                cx.reader()
                    .write_file(path, &restored, Privilege::User)
                    .map_err(StepError::Exec)?;
            }
            // Listed rather than wildcarded, so a change a caller starts
            // recording fails to compile here instead of being silently
            // skipped by rollback.
            Change::FileWritten { .. }
            | Change::FileRemoved { .. }
            | Change::SysctlRuntime { .. }
            | Change::SysfsWrite { .. }
            | Change::PackagesInstalled { .. }
            | Change::SystemdUnit { .. }
            | Change::DirCreated { .. }
            | Change::DirTreeInstalled { .. } => {}
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "restore_steam_config_test.rs"]
mod restore_steam_config_test;
