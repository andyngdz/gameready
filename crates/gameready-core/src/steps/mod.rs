//! The improvements gameready ships.

mod constants;
pub mod domain;
mod service;
mod use_cases;

pub use constants::{
    COMPAT_MAPPING_PATH, COMPAT_NAME_KEY, COMPAT_TOOLS_DIR, COMPAT_TOOL_VDF, LAUNCH_OPTIONS_KEY,
    MANAGED_HEADER, MKDIR_BIN, PROC_SWAPS, PROTON_EXPERIMENTAL, PROTON_GE_LATEST_URL, RMDIR_BIN,
    STEAM_APPS_PATH, SYSCTL_BIN, SYSCTL_DROPIN,
};
pub use domain::{
    apply_targets, newest_ge_proton, resolve_wishes, CompatRank, CompatTarget, CompatWish,
    CompetingDaemon, Edited, GamingTool, LaunchTarget, COMPETING_DAEMONS, GAMING_TOOLS,
};
pub use service::{core_steps, find_core_step, game_steps};
pub(crate) use use_cases::{disk_inventory, DiskInventory};
pub use use_cases::{
    Conflicts, CpuGovernor, GamingTools, IoScheduler, MaxMapCount, SteamLaunchOptions, SteamProton,
};
