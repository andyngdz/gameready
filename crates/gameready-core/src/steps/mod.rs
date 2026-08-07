//! The improvements gameready ships.

mod constants;
pub mod domain;
mod service;
mod use_cases;

pub use constants::{
    ADD_APT_REPOSITORY_BIN, APT_ASSUME_YES, APT_REMOVE, LAUNCH_OPTIONS_KEY, MANAGED_HEADER,
    SCXCTL_BIN, STEAM_APPS_PATH, SYSCTL_BIN, SYSCTL_DROPIN,
};
pub use domain::{
    COMPETING_DAEMONS, CompetingDaemon, Edited, GAMING_TOOLS, GamingTool, LaunchTarget,
    apply_targets, restore_scheduler,
};
pub use service::{core_steps, find_core_step};
pub use use_cases::{
    Conflicts, CpuGovernor, GamingTools, IoScheduler, MaxMapCount, ScxLavd, ScxPpa,
    SteamLaunchOptions,
};
