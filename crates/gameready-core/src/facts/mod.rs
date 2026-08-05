//! Probing the machine gameready is running on.

mod constants;
mod domain;
mod errors;
mod service;

pub use domain::{Distro, Family, PackageManagerKind, RootFilesystem, SystemFacts};
pub use errors::FactsError;
pub use service::{OS_RELEASE, parse_kernel_release, parse_os_release, probe};
