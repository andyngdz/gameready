//! The parts every single-key sysctl step repeats.
//!
//! Extracted once `core.sysctl.split-lock` became a second caller: reading a
//! numeric parameter, rendering the drop-in that persists it, and undoing the
//! pair are the same work whichever key is involved.

use std::path::Path;

use crate::exec::CommandRunner;
use crate::improvement::{ImprovementId, ParseFailure, StepError};
use crate::journal::RunId;
use crate::steps::constants::managed_header;

/// Reads a kernel parameter's live value.
///
/// A value that will not parse is an error, never a default: a step that cannot
/// read the current state cannot restore it, so falling through to "apply"
/// would make the change unrecoverable.
pub fn read_value(
    runner: &dyn CommandRunner,
    path: &Path,
    key: &'static str,
) -> Result<u64, StepError> {
    let raw = runner.read_to_string(path).map_err(StepError::Exec)?;

    raw.trim()
        .parse::<u64>()
        .map_err(|source| StepError::Parse {
            what: key,
            path: path.to_path_buf(),
            source: ParseFailure::Integer(source),
        })
}

/// The drop-in that makes one parameter survive a reboot.
///
/// The run id has to be the run, not the step: `doctor` uses it to tie a
/// leftover file back to the invocation that created it when the journal has
/// been deleted.
#[must_use]
pub fn single_key_dropin(step: ImprovementId, run: RunId, key: &str, target: u64) -> String {
    format!(
        "{header}\n\
         # Remove this file or run `gameready rollback` to revert.\n\
         {key} = {target}\n",
        header = managed_header(step, run),
    )
}

#[cfg(test)]
#[path = "sysctl_dropin_test.rs"]
mod sysctl_dropin_test;
