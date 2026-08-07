//! Reading what Steam has installed, and what gameready can do for it.

mod domain;
mod errors;
mod service;

pub use domain::{is_valve_tool, InstalledGame};
pub use errors::{SteamError, VdfError};
pub use service::{
    pair_with_catalog, set_block, set_scalar, with_overlay, Edit, GameSetup, Overlay, SetResult,
};
