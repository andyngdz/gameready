//! Tables the built-in steps read from.

mod block_device;
mod compat;
mod daemons;
mod launch;
mod proton_ge;
mod sched_ext;
mod swap;
mod tools;

pub use block_device::{
    is_tunable_disk, parse_scheduler_line, BlockDevice, SchedulerState, NVME_SCHEDULER,
    ROTATIONAL_SCHEDULER, SSD_SCHEDULER,
};
pub use compat::{apply_compat_targets, CompatEdited, CompatTarget};
pub use daemons::{CompetingDaemon, COMPETING_DAEMONS, GOVERNOR_DAEMONS};
pub use launch::{apply_targets, Edited, LaunchTarget};
pub use proton_ge::{newest_ge_proton, parse_checksum, parse_release, tarball_name, ProtonRelease};
pub use sched_ext::{load_scheduler, restore_scheduler, SchedExt, SCX_SCHEDS, SCX_TOOLS};
pub use swap::{active_swap, parse_proc_swaps, primary_is_zram, ActiveSwap, SwapArea, SwapBacking};
pub use tools::{GamingTool, GAMEMODE, GAMING_TOOLS};
