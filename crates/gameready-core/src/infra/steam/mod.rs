//! Reading and writing a real Steam installation.

mod config;
mod process;
mod scan;
mod setup;
mod write_launch;

pub use config::{local_config_under, locate_local_config};
pub use process::{is_running, shutdown};
pub use scan::{scan_installed_games, scan_installed_games_in};
pub use setup::discover_setups;
pub use write_launch::write_launch_options;
