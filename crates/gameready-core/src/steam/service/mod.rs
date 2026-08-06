//! Matching what is installed against what gameready knows how to tune.

mod pairing;
mod vdf;

pub use pairing::{GameSetup, Overlay, pair_with_catalog, with_overlay};
pub use vdf::{Edit, SetResult, set_scalar};
