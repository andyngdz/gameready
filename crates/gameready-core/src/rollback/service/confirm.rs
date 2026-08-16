//! Reading the system back after an undo says it worked.

use std::path::Path;

use crate::exec::CommandRunner;
use crate::journal::{PriorUnitState, Undo};
use crate::steam::{sections_match, PriorSection};
use crate::systemd::{unit_state, UnitState};

/// The `/proc` tree a sysctl key is readable under.
const PROC_SYS: &str = "/proc/sys";

/// Checks that the system really is what the undo claimed to make it.
///
/// A command that exits zero has reported its own success, which is not the
/// same as the machine having changed. `sysctl -w` on a key the kernel no
/// longer has, a write that lands in a container's masked `/proc`, a systemd
/// unit that accepts `disable` and stays running: each of those exits zero and
/// leaves the setting where it was. Rollback is the one place where believing
/// that would tell a user their machine is back to normal when it is not.
///
/// `None` means confirmed. `Some(reason)` is what to tell the user instead.
/// Only meaningful for an undo that reported [`UndoOutcome::Reverted`], since
/// every other outcome already says nothing was changed.
///
/// [`UndoOutcome::Reverted`]: crate::rollback::UndoOutcome::Reverted
pub(super) fn confirm(undo: &Undo, runner: &dyn CommandRunner) -> Option<String> {
    match undo {
        Undo::DeleteFile { path, .. }
        | Undo::RemoveDirIfEmpty { path, .. }
        | Undo::RemoveDirTree { path, .. } => gone(runner, path),

        Undo::RestoreSteamConfig { path, sections } => {
            steam_config_put_back(runner, path, sections)
        }

        Undo::SetSysctl { key, value } => sysctl_reads_back(runner, key, value),

        Undo::WriteSysfs { path, value } => sysfs_reads_back(runner, path, value),

        Undo::RestoreUnit { unit, prior } => unit_is_back(runner, unit, *prior),

        // Reporting packages changes nothing, so there is nothing to read back.
        // It never reports Reverted either, which is what this runs after.
        Undo::ReportPackages { .. } => None,
    }
}

/// Confirms a path the undo removed is not there any more.
fn gone(runner: &dyn CommandRunner, path: &Path) -> Option<String> {
    runner
        .path_exists(path)
        .then(|| format!("{} is still on disk", path.display()))
}

/// Confirms the recorded keys read back as recorded.
///
/// Compared by value, not by text: a restore re-renders the document and the
/// parser normalises indentation, so a correctly restored file does not match
/// the original byte for byte and never will.
fn steam_config_put_back(
    runner: &dyn CommandRunner,
    path: &Path,
    sections: &[PriorSection],
) -> Option<String> {
    let current = runner.read_to_string(path).ok()?;
    match sections_match(&current, sections) {
        Ok(true) => None,
        // The undo claimed to have put the recorded keys back and they do not
        // read back that way. The message stays on what the record promised
        // rather than guessing which side moved: Steam may have dropped a block
        // the run wrote, or recreated one the undo removed.
        Ok(false) => Some(format!(
            "{} does not read back the keys the run recorded",
            path.display()
        )),
        Err(error) => Some(error.to_string()),
    }
}

/// Confirms a kernel parameter holds the value the undo put back.
///
/// Read from `/proc/sys` rather than by running `sysctl -n`, so the check does
/// not go through the same tool whose exit code is the thing in doubt.
fn sysctl_reads_back(runner: &dyn CommandRunner, key: &str, value: &str) -> Option<String> {
    let path = Path::new(PROC_SYS).join(key.replace('.', "/"));
    let current = runner.read_to_string(&path).ok()?;
    (current.trim() != value.trim()).then(|| format!("{key} reads {}, not {value}", current.trim()))
}

/// Confirms a sysfs attribute holds the value the undo put back.
///
/// A queue scheduler prints every choice with the live one in brackets, so the
/// answer is checked by containing `[value]` rather than by equality.
fn sysfs_reads_back(runner: &dyn CommandRunner, path: &Path, value: &str) -> Option<String> {
    let current = runner.read_to_string(path).ok()?;
    let current = current.trim();
    let selected = current.contains(&format!("[{}]", value.trim())) || current == value.trim();
    (!selected).then(|| format!("{} reads {current}, not {value}", path.display()))
}

/// Confirms a unit is back in the state the run found it in.
///
/// Both sides are worded by [`UnitState::describe`], so the row reads the same
/// way the rest of the tool talks about units.
fn unit_is_back(runner: &dyn CommandRunner, unit: &str, prior: PriorUnitState) -> Option<String> {
    let state = unit_state(runner, unit).ok()?;
    let wanted_running = prior == PriorUnitState::WasEnabled;
    if (state == UnitState::Running) == wanted_running {
        return None;
    }

    let expected = if wanted_running {
        UnitState::Running
    } else {
        UnitState::Dormant
    };
    Some(format!(
        "{unit} is {}; the run found it {}",
        state.describe(),
        expected.describe()
    ))
}

#[cfg(test)]
#[path = "confirm_test.rs"]
mod confirm_test;
