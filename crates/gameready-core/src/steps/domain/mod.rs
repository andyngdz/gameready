//! Tables the built-in steps read from.

mod daemons;
mod launch;
mod tools;

pub use daemons::{COMPETING_DAEMONS, CompetingDaemon};
pub use launch::{Edited, LaunchTarget, apply_targets};
pub use tools::{GAMING_TOOLS, GamingTool};
