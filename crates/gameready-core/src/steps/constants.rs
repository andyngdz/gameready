//! Paths and markers shared by the built-in steps.

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

/// Kernel parameter some Proton titles need raised before they will start.
pub const VM_MAX_MAP_COUNT: &str = "vm.max_map_count";

/// Where the kernel exposes the live value of a `vm.` parameter.
pub const PROC_SYS_VM: &str = "/proc/sys/vm";

/// The tool that reads and writes kernel parameters.
pub const SYSCTL_BIN: &str = "sysctl";

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
