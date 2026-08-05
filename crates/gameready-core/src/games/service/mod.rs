//! Reading profiles and assembling them into a catalog.

mod catalog;
mod parse;
mod schema;

pub use catalog::{Catalog, CatalogEntry, Source};
pub use parse::parse_profile;
