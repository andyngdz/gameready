//! Reading profiles and assembling them into a catalog.

mod catalog;
mod launch;
mod parse;
mod schema;

pub use catalog::{Catalog, CatalogEntry, Source};
pub use launch::launch_options;
pub use parse::parse_profile;
