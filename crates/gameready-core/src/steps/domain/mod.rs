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
    BlockDevice, NVME_SCHEDULER, ROTATIONAL_SCHEDULER, SSD_SCHEDULER, SchedulerState,
    is_tunable_disk, parse_scheduler_line,
};
pub use compat::{CompatEdited, CompatTarget, apply_compat_targets};
pub use daemons::{COMPETING_DAEMONS, CompetingDaemon};
pub use launch::{Edited, LaunchTarget, apply_targets};
pub use proton_ge::{ProtonRelease, newest_ge_proton, parse_checksum, parse_release, tarball_name};
pub use sched_ext::{SCX_SCHEDS, SCX_TOOLS, SchedExt, load_scheduler, restore_scheduler};
pub use swap::{SwapArea, SwapBacking, parse_proc_swaps, primary_is_zram};
pub use tools::{GAMING_TOOLS, GamingTool};
