//! What the doctor screen says about the machine itself.

use std::path::Path;

use crate::exec::CommandRunner;
use crate::steps::domain::{active_swap, parse_proc_swaps, ActiveSwap};
use crate::steps::{disk_inventory, DiskInventory, PROC_SWAPS};

/// The machine facts the doctor screen reports beyond distro and kernel.
///
/// Owned and read-only: doctor states what is there, so nothing here carries a
/// target or a plan.
pub struct MachineReport {
    /// The active swap, or `None` when the machine has none. Its backing is why
    /// the swappiness step applies or not.
    pub swap: Option<ActiveSwap>,

    /// Every tunable disk with the scheduler it is on now.
    pub disks: Vec<DiskInventory>,
}

/// Reads the machine facts the "Your machine" block reports.
#[must_use]
pub fn machine_report(runner: &dyn CommandRunner) -> MachineReport {
    let swaps = runner
        .read_to_string(Path::new(PROC_SWAPS))
        .ok()
        .map(|raw| parse_proc_swaps(&raw))
        .unwrap_or_default();

    MachineReport {
        swap: active_swap(&swaps),
        disks: disk_inventory(runner),
    }
}

#[cfg(test)]
#[path = "machine_test.rs"]
mod machine_test;
