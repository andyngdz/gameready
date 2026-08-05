//! One module per built-in improvement.

mod conflicts;
mod cpu_governor;
mod gaming_tools;
mod sysctl_max_map_count;

pub use conflicts::Conflicts;
pub use cpu_governor::CpuGovernor;
pub use gaming_tools::GamingTools;
pub use sysctl_max_map_count::MaxMapCount;
