//! Reading the tunable disks under `/sys/block` and their schedulers.

use std::path::{Path, PathBuf};

use itertools::Itertools as _;

use crate::exec::CommandRunner;
use crate::improvement::StepError;
use crate::steps::constants::{QUEUE_ROTATIONAL, QUEUE_SCHEDULER, SYS_BLOCK};
use crate::steps::domain::{is_tunable_disk, parse_scheduler_line, BlockDevice, SchedulerState};

/// One disk, where its scheduler lives, its current state, and its target.
pub(super) struct DiskScheduler {
    pub(super) name: String,
    pub(super) scheduler_path: PathBuf,
    pub(super) state: SchedulerState,
    pub(super) target: &'static str,
}

impl DiskScheduler {
    /// Whether this disk should be switched: a real difference, and a target
    /// the kernel offers.
    pub(super) fn needs_change(&self) -> bool {
        self.state.offers(self.target) && self.state.active != self.target
    }
}

/// Reads every tunable disk under `/sys/block` with its scheduler state.
///
/// A disk whose flag or scheduler cannot be read is skipped rather than failing
/// the whole step: one odd entry must not stop the rest from being tuned. A
/// failure to list `/sys/block` itself is an error, since then nothing is known.
pub(super) fn scan_disks(runner: &dyn CommandRunner) -> Result<Vec<DiskScheduler>, StepError> {
    let entries = runner
        .read_dir(Path::new(SYS_BLOCK))
        .map_err(StepError::Exec)?;

    let mut disks = Vec::new();
    for entry in entries {
        let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !is_tunable_disk(name) {
            continue;
        }
        let Some(rotational) = read_rotational(runner, &entry) else {
            continue;
        };
        let scheduler_path = entry.join(QUEUE_SCHEDULER);
        let Some(state) = runner
            .read_to_string(&scheduler_path)
            .ok()
            .and_then(|line| parse_scheduler_line(&line))
        else {
            continue;
        };
        let target = BlockDevice {
            name: name.to_owned(),
            rotational,
        }
        .target_scheduler();
        disks.push(DiskScheduler {
            name: name.to_owned(),
            scheduler_path,
            state,
            target,
        });
    }
    Ok(disks)
}

/// Whether the kernel reports the device at `entry` as a spinning disk.
fn read_rotational(runner: &dyn CommandRunner, entry: &Path) -> Option<bool> {
    let raw = runner.read_to_string(&entry.join(QUEUE_ROTATIONAL)).ok()?;
    Some(raw.trim() == "1")
}

/// One disk and the scheduler it is on now, for the doctor screen.
///
/// Owned and free of the target/change machinery so it can cross out of this
/// feature: doctor only wants to report what is there, not what would change.
pub struct DiskInventory {
    pub name: String,
    pub scheduler: String,
}

/// Every tunable disk with its active scheduler, for the doctor screen.
///
/// A `/sys/block` that cannot be read lists nothing rather than failing, since
/// doctor reports rather than acts.
#[must_use]
pub(crate) fn disk_inventory(runner: &dyn CommandRunner) -> Vec<DiskInventory> {
    scan_disks(runner)
        .map(|disks| {
            disks
                .into_iter()
                .map(|disk| DiskInventory {
                    name: disk.name,
                    scheduler: disk.state.active,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A one-line summary of what will change, for the plan screen.
pub(super) fn summary(disks: &[DiskScheduler]) -> String {
    let changing = disks
        .iter()
        .filter(|disk| disk.needs_change())
        .map(|disk| format!("{} -> {}", disk.name, disk.target))
        .join(", ");
    if changing.is_empty() {
        "I/O scheduler already set for every disk".to_owned()
    } else {
        format!("I/O scheduler: {changing}")
    }
}

#[cfg(test)]
#[path = "io_scheduler_devices_test.rs"]
mod io_scheduler_devices_test;
