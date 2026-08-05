//! Driving a system's package manager.

mod domain;
mod errors;
mod traits;

pub use domain::{InstallOutcome, PackageState};
pub use errors::PackageError;
pub use traits::PackageManager;
