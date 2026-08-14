//! Matching what is installed against what gameready knows how to tune.

mod pairing;
mod vdf;
mod vdf_prior;

pub use pairing::{pair_with_catalog, with_overlay, GameSetup, Overlay};
pub use vdf::{set_block, set_scalar, Edit, SetResult};
pub use vdf_prior::{
    capture_block, restore_block, restore_sections, sections_match, PriorBlock, PriorScalar,
    PriorSection,
};
