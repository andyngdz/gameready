//! Reversing a change made inside a config file Steam owns.

use std::path::Path;

use crate::exec::CommandRunner;
use crate::improvement::Privilege;
use crate::rollback::domain::UndoOutcome;
use crate::steam::{restore_sections, sections_match, PriorBlock, PriorSection};

/// Puts back the keys a run set, leaving everything Steam wrote since.
///
/// The file is read at undo time rather than restored from a pre-image, because
/// Steam saves it on every exit. A pre-image would be correct about the keys
/// gameready set and wrong about everything else in a file the user has been
/// using ever since.
///
/// Written as the user: both files live in the user's home, and a root-owned
/// copy would stop Steam from saving its own settings.
pub(super) fn restore_steam_config(
    runner: &dyn CommandRunner,
    path: &Path,
    sections: &[PriorSection],
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

    // Restoring would rewrite the whole document even when there is nothing to
    // change, so the recorded keys are checked first: an already-rolled-back
    // file must not be rewritten just because its formatting no longer matches
    // what this undo would render.
    if sections_match(&current, sections).unwrap_or(false) {
        return UndoOutcome::AlreadyGone;
    }

    let restored = match restore_sections(&current, sections) {
        Ok(restored) => restored,
        Err(error) => {
            return UndoOutcome::Failed {
                error: error.to_string(),
            };
        }
    };

    // Restoring the keys this file already holds is a no-op, not a failure.
    if restored == current {
        return UndoOutcome::AlreadyGone;
    }

    match runner.write_file(path, &restored, Privilege::User) {
        Ok(()) => UndoOutcome::Reverted {
            detail: format!("put back {} in {}", describe(sections), path.display()),
        },
        Err(error) => UndoOutcome::Failed {
            error: error.to_string(),
        },
    }
}

/// Names what the undo put back, for the row the user reads.
fn describe(sections: &[PriorSection]) -> String {
    let added = sections
        .iter()
        .filter(|section| section.prior == PriorBlock::Absent)
        .count();
    match (added, sections.len()) {
        (0, total) => format!("{total} setting(s)"),
        (added, total) if added == total => format!("{added} entry(s) gameready added"),
        (added, total) => format!("{total} setting(s), {added} of them added by gameready"),
    }
}

#[cfg(test)]
#[path = "perform_steam_test.rs"]
mod perform_steam_test;
