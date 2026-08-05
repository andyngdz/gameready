//! Probing the machine gameready is running on.

mod domain;
mod errors;
mod service;

pub use domain::SystemFacts;
pub use errors::FactsError;
pub use service::{parse_kernel_release, probe};
