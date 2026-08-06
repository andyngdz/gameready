//! One module per built-in improvement.

mod conflicts;
mod cpu_governor;
mod gaming_tools;
mod io_scheduler;
mod io_scheduler_devices;
mod steam_launch_options;
mod sysctl_max_map_count;

pub use conflicts::Conflicts;
pub use cpu_governor::CpuGovernor;
pub use gaming_tools::GamingTools;
pub use io_scheduler::IoScheduler;
pub use steam_launch_options::SteamLaunchOptions;
pub use sysctl_max_map_count::MaxMapCount;
