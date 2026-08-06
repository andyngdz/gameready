//! Per-game profiles: what they say, and which ones this machine can see.
//!
//! Adding a game is adding a `games/<Name>/game.toml`. A game that needs logic
//! no profile can express names a Rust module in its `[override]` table.

mod domain;
mod errors;
mod service;

pub use domain::{AppId, GameKey, GameProfile, GameRef, ProtonChoice, Wrapper};
pub use errors::GameError;
pub use service::{Catalog, CatalogEntry, Source, default_wrappers, launch_options, parse_profile};
