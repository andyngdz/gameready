//! Reading what Steam has installed, and what gameready can do for it.

mod domain;
mod errors;
mod service;

pub use domain::{InstalledGame, is_valve_tool};
pub use errors::{SteamError, VdfError};
pub use service::{
    Edit, GameSetup, Overlay, SetResult, pair_with_catalog, set_block, set_scalar, with_overlay,
};
