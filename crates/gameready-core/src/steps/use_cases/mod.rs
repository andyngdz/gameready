//! One module per built-in improvement.

mod conflicts;
mod cpu_governor;
mod cpu_governor_policies;
mod gamemode_config;
mod gamemode_config_file;
mod gamemode_config_group;
mod gaming_tools;
mod gaming_tools_survey;
mod gpu_shader_cache;
mod gpu_shader_cache_fragment;
mod gpu_shader_cache_vendor;
mod io_scheduler;
mod io_scheduler_devices;
mod memory_swappiness;
mod memory_swappiness_state;
mod proton_ge;
mod restore_steam_config;
mod steam_launch_options;
mod steam_proton;
mod sysctl_dropin;
mod sysctl_max_map_count;
mod sysctl_split_lock;
mod sysctl_vm_latency;
mod sysctl_vm_latency_survey;
mod user_home;

pub(crate) use io_scheduler_devices::{disk_inventory, DiskInventory};

pub use conflicts::Conflicts;
pub use cpu_governor::CpuGovernor;
pub use gamemode_config::GamemodeConfig;
pub use gaming_tools::GamingTools;
pub use gpu_shader_cache::ShaderCache;
pub use io_scheduler::IoScheduler;
pub use memory_swappiness::Swappiness;
pub use proton_ge::ProtonGe;
pub use steam_launch_options::SteamLaunchOptions;
pub use steam_proton::SteamProton;
pub use sysctl_max_map_count::MaxMapCount;
pub use sysctl_split_lock::SplitLock;
pub use sysctl_vm_latency::VmLatency;
