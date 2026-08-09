//! Paths and markers shared by the built-in steps.
//!
//! Split by the subsystem each constant belongs to rather than kept in one
//! list, so a step that touches only sysctl does not have to read past the
//! Steam and scx vocabularies to find what it needs.

mod block;
mod scx;
mod session;
mod steam;
mod sysctl;

pub use block::{IO_SCHEDULER_RULE, QUEUE_ROTATIONAL, QUEUE_SCHEDULER, SYS_BLOCK};
pub use scx::{
    ADD_APT_REPOSITORY_BIN, APT_ASSUME_YES, APT_REMOVE, LAVD_SCHEDULER, SCHED_EXT_DISABLED,
    SCHED_EXT_OPS, SCHED_EXT_STATE, SCXCTL_BIN, SCX_LAVD_BIN, SCX_PPA, SCX_PPA_ORIGIN, SCX_PPA_PIN,
    SCX_SCHEDULER_OVERRIDE, SCX_SERVICE_NAME, SCX_UNIT_DROPIN, SCX_UNIT_PATH,
};
pub use session::{
    DEVICE_VENDOR, ENVIRONMENT_D_DIR, GAMEMODE_INI, SHADER_CACHE_CONF, SYS_CLASS_DRM,
};
pub use steam::{
    COMPAT_CONFIG_KEY, COMPAT_MAPPING_PATH, COMPAT_NAME_KEY, COMPAT_PRIORITY, COMPAT_PRIORITY_KEY,
    COMPAT_TOOLS_DIR, COMPAT_TOOL_VDF, CONFIG_BACKUP, CURL_BIN, LAUNCH_OPTIONS_KEY,
    LOCAL_CONFIG_BACKUP, PROTON_EXPERIMENTAL, PROTON_GE_LATEST_URL, SHA512SUM_BIN, STEAM_APPS_PATH,
    TAR_BIN,
};
pub use sysctl::{
    KERNEL_SPLIT_LOCK_MITIGATE, PROC_SWAPS, PROC_SYS_KERNEL, PROC_SYS_VM, SPLIT_LOCK_DROPIN,
    SWAPPINESS_DROPIN, SYSCTL_BIN, SYSCTL_DROPIN, VM_LATENCY_DROPIN, VM_MAX_MAP_COUNT,
    VM_SWAPPINESS,
};

use crate::improvement::ImprovementId;
use crate::journal::RunId;

/// Marks a file as gameready's own.
///
/// `doctor` scans for this so it can find and clean up leftovers even when the
/// journal has been deleted, which is the failure mode of keeping state in
/// `$HOME`. Formatted with the tool version, the step id, and the run id.
pub const MANAGED_HEADER: &str = "# Managed by gameready";

/// What a verification check reports for a value that is not there yet.
pub const NOT_SET: &str = "not set";

/// The reassurance an undo carries when it takes effect immediately.
///
/// Shared so every step that can promise it words the promise identically; a
/// step whose undo needs a reboot must not reach for this.
pub const UNDO_NO_REBOOT: &str = "no reboot";

/// The tool that creates a directory and its parents.
pub const MKDIR_BIN: &str = "mkdir";

/// The tool that removes a directory only when nothing is left in it.
///
/// Used instead of `rm -r` wherever gameready gives back a directory it made:
/// its refusal to touch a non-empty directory is the safety property, not an
/// inconvenience, since these are shared XDG paths other software also writes.
pub const RMDIR_BIN: &str = "rmdir";

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

#[cfg(test)]
#[path = "constants_test.rs"]
mod constants_test;
