//! Reading and writing a real Steam installation.

mod appinfo;
mod config;
mod process;
mod scan;
mod setup;
mod undo_settings;
mod write_settings;

pub use config::{
    configs_under, install_config_under, installed_compat_tools, local_config_under,
    locate_local_config, locate_steam_dir, SteamConfigs,
};
pub use process::{is_running, shutdown, start};
pub use scan::{scan_installed_games, scan_installed_games_in};
pub use setup::discover_setups;
pub use undo_settings::undo_with_steam_closed;
pub use write_settings::{write_steam_settings, SteamSettings};
