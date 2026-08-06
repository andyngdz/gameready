//! Paths and markers shared by the built-in steps.

use crate::improvement::ImprovementId;
use crate::journal::RunId;

/// Every sysctl gameready sets goes in this one file.
///
/// One file, always created and never edited, is what keeps the undo simple:
/// there is no prior content to preserve, so losing the journal still leaves a
/// change that is identifiable and removable. Editing `/etc/sysctl.conf` in
/// place would make rollback depend on a backup that may not exist.
pub const SYSCTL_DROPIN: &str = "/etc/sysctl.d/99-gameready.conf";

/// Marks a file as gameready's own.
///
/// `doctor` scans for this so it can find and clean up leftovers even when the
/// journal has been deleted, which is the failure mode of keeping state in
/// `$HOME`. Formatted with the tool version, the step id, and the run id.
pub const MANAGED_HEADER: &str = "# Managed by gameready";

/// The first line of every file gameready manages.
///
/// Carries [`MANAGED_HEADER`] so `doctor` can find the file after the journal
/// is gone, plus the version, step, and run that wrote it. One builder so every
/// step that manages an `/etc` file stamps the marker identically, and the
/// package version is read in exactly one place.
#[must_use]
pub fn managed_header(step: ImprovementId, run: RunId) -> String {
    format!(
        "{MANAGED_HEADER} {version} - step={step} run={run}",
        version = env!("CARGO_PKG_VERSION"),
    )
}

/// Kernel parameter some Proton titles need raised before they will start.
pub const VM_MAX_MAP_COUNT: &str = "vm.max_map_count";

/// Where the kernel exposes the live value of a `vm.` parameter.
pub const PROC_SYS_VM: &str = "/proc/sys/vm";

/// The tool that reads and writes kernel parameters.
pub const SYSCTL_BIN: &str = "sysctl";

/// The directory the kernel lists every whole block device under.
pub const SYS_BLOCK: &str = "/sys/block";

/// A block device's scheduler attribute, relative to its `/sys/block` entry.
pub const QUEUE_SCHEDULER: &str = "queue/scheduler";

/// A block device's rotational flag, relative to its `/sys/block` entry.
pub const QUEUE_ROTATIONAL: &str = "queue/rotational";

/// The udev rule that makes the scheduler choice survive a reboot.
///
/// Its own file, never an edit of an existing one, so losing the journal still
/// leaves a change that is identifiable and removable.
pub const IO_SCHEDULER_RULE: &str = "/etc/udev/rules.d/60-gameready-ioscheduler.rules";

/// Where the kernel reports the first core's frequency-scaling governor.
///
/// Core zero stands in for the machine: the governor is per-policy, and a
/// system with different governors on different cores was configured that way
/// by hand and is not one gameready should be second-guessing.
pub const SCALING_GOVERNOR: &str = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor";

/// Where Steam keeps per-game settings inside `localconfig.vdf`.
pub const STEAM_APPS_PATH: [&str; 5] =
    ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];

/// The key holding a game's launch options.
pub const LAUNCH_OPTIONS_KEY: &str = "LaunchOptions";

/// The name the pre-image of Steam's config is filed under in a run's backups.
pub const LOCAL_CONFIG_BACKUP: &str = "localconfig.vdf";

#[cfg(test)]
#[path = "constants_test.rs"]
mod constants_test;
