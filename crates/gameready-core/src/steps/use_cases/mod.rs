//! One module per built-in improvement.

mod conflicts;
mod cpu_governor;
mod gaming_tools;
mod gaming_tools_survey;
mod io_scheduler;
mod io_scheduler_devices;
mod memory_swappiness;
mod proton_ge;
mod scx_lavd;
mod scx_lavd_loader;
mod scx_lavd_packages;
mod scx_ppa;
mod scx_ppa_pin;
mod scx_state;
mod steam_launch_options;
mod sysctl_max_map_count;

pub use conflicts::Conflicts;
pub use cpu_governor::CpuGovernor;
pub use gaming_tools::GamingTools;
pub use io_scheduler::IoScheduler;
pub use memory_swappiness::Swappiness;
pub use proton_ge::ProtonGe;
pub use scx_lavd::ScxLavd;
pub use scx_ppa::ScxPpa;
pub use steam_launch_options::SteamLaunchOptions;
pub use sysctl_max_map_count::MaxMapCount;
