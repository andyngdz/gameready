//! Matching what is installed against what gameready knows how to tune.

mod pairing;
mod vdf;

pub use pairing::{pair_with_catalog, with_overlay, GameSetup, Overlay};
pub use vdf::{set_block, set_scalar, Edit, SetResult};
