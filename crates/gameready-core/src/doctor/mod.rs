//! System health checks and warnings.

pub mod domain;
pub(crate) mod use_cases;

pub use domain::Warning;
pub use use_cases::check_warnings::check_warnings;
