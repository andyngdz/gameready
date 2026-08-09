//! Kernel parameters, the files that make them survive a reboot, and the tool
//! that sets them.

/// Every sysctl gameready sets goes in this one file.
///
/// One file, always created and never edited, is what keeps the undo simple:
/// there is no prior content to preserve, so losing the journal still leaves a
/// change that is identifiable and removable. Editing `/etc/sysctl.conf` in
/// place would make rollback depend on a backup that may not exist.
pub const SYSCTL_DROPIN: &str = "/etc/sysctl.d/99-gameready.conf";

/// Kernel parameter some Proton titles need raised before they will start.
pub const VM_MAX_MAP_COUNT: &str = "vm.max_map_count";

/// Kernel parameter that decides how eagerly pages are swapped out.
pub const VM_SWAPPINESS: &str = "vm.swappiness";

/// The swappiness drop-in, kept separate from [`SYSCTL_DROPIN`].
///
/// Its own file so each sysctl step owns exactly one created-never-edited file:
/// writing both keys into one file would make the two steps clobber each
/// other's contents and each other's rollback.
pub const SWAPPINESS_DROPIN: &str = "/etc/sysctl.d/99-gameready-swappiness.conf";

/// Kernel parameter that decides how a split lock is punished.
pub const KERNEL_SPLIT_LOCK_MITIGATE: &str = "kernel.split_lock_mitigate";

/// The split-lock drop-in, its own file for the reason given on
/// [`SWAPPINESS_DROPIN`].
pub const SPLIT_LOCK_DROPIN: &str = "/etc/sysctl.d/99-gameready-splitlock.conf";

/// The allocation-latency drop-in, its own file for the same reason.
pub const VM_LATENCY_DROPIN: &str = "/etc/sysctl.d/99-gameready-vm-latency.conf";

/// Where the kernel lists every active swap area.
pub const PROC_SWAPS: &str = "/proc/swaps";

/// Where the kernel exposes the live value of a `vm.` parameter.
pub const PROC_SYS_VM: &str = "/proc/sys/vm";

/// Where the kernel exposes the live value of a `kernel.` parameter.
pub const PROC_SYS_KERNEL: &str = "/proc/sys/kernel";

/// The tool that reads and writes kernel parameters.
pub const SYSCTL_BIN: &str = "sysctl";
