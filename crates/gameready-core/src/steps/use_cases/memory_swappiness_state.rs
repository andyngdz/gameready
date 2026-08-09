//! Reading what the kernel currently reports about swap.

use std::path::{Path, PathBuf};

use crate::exec::CommandRunner;
use crate::improvement::{ParseFailure, StepError};
use crate::steps::constants::{PROC_SWAPS, PROC_SYS_VM, VM_SWAPPINESS};
use crate::steps::domain::{parse_proc_swaps, SwapArea};

/// Where the kernel exposes the live value.
pub(super) fn runtime_path() -> PathBuf {
    Path::new(PROC_SYS_VM).join("swappiness")
}

/// The active swap areas, empty when `/proc/swaps` cannot be read: an
/// unknown swap layout means `NotApplicable`, not a guess at swappiness.
pub(super) fn read_swaps(runner: &dyn CommandRunner) -> Vec<SwapArea> {
    runner
        .read_to_string(Path::new(PROC_SWAPS))
        .ok()
        .map(|raw| parse_proc_swaps(&raw))
        .unwrap_or_default()
}

/// Reads the value the kernel currently reports.
pub(super) fn read_current(runner: &dyn CommandRunner) -> Result<u16, StepError> {
    let path = runtime_path();
    let raw = runner.read_to_string(&path).map_err(StepError::Exec)?;

    raw.trim()
        .parse::<u16>()
        .map_err(|source| StepError::Parse {
            what: VM_SWAPPINESS,
            path,
            source: ParseFailure::Integer(source),
        })
}

#[cfg(test)]
#[path = "memory_swappiness_state_test.rs"]
mod memory_swappiness_state_test;
