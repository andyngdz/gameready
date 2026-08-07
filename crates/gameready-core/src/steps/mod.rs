//! The improvements gameready ships.

mod constants;
pub mod domain;
mod service;
mod use_cases;

pub use constants::{
    ADD_APT_REPOSITORY_BIN, APT_ASSUME_YES, APT_REMOVE, COMPAT_MAPPING_PATH, COMPAT_NAME_KEY,
    COMPAT_TOOLS_DIR, COMPAT_TOOL_VDF, LAUNCH_OPTIONS_KEY, MANAGED_HEADER, PROTON_EXPERIMENTAL,
    SCXCTL_BIN, STEAM_APPS_PATH, SYSCTL_BIN, SYSCTL_DROPIN,
};
pub use domain::{
    apply_targets, newest_ge_proton, restore_scheduler, CompatTarget, CompetingDaemon, Edited,
    GamingTool, LaunchTarget, COMPETING_DAEMONS, GAMING_TOOLS,
};
pub use service::{core_steps, find_core_step};
pub use use_cases::{
    Conflicts, CpuGovernor, GamingTools, IoScheduler, MaxMapCount, ScxLavd, ScxPpa,
    SteamLaunchOptions, SteamProton,
};
