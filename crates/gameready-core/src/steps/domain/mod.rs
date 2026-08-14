//! Tables the built-in steps read from.

mod block_device;
mod compat;
mod daemons;
mod gpu;
mod launch;
mod proton_ge;
mod swap;
mod tools;
mod vm_latency;

pub use block_device::{
    is_tunable_disk, parse_scheduler_line, BlockDevice, SchedulerState, NVME_SCHEDULER,
    ROTATIONAL_SCHEDULER, SSD_SCHEDULER,
};
pub use compat::{
    apply_compat_targets, resolve_wishes, CompatEdited, CompatRank, CompatTarget, CompatWish,
};
pub use daemons::{CompetingDaemon, COMPETING_DAEMONS, GOVERNOR_DAEMONS};
pub use gpu::{CacheSetting, DetectedGpu, GpuVendor};
pub use launch::{apply_targets, Edited, LaunchTarget};
pub use proton_ge::{newest_ge_proton, parse_checksum, parse_release, ProtonRelease};
pub use swap::{active_swap, parse_proc_swaps, primary_is_zram, ActiveSwap, SwapArea, SwapBacking};
pub use tools::{GamingTool, GAMEMODE, GAMING_TOOLS};
pub use vm_latency::{LatencyKnob, VM_LATENCY_KNOBS};
