//! What gameready knows about the machine it is running on.

pub(crate) mod distro;
mod system_facts;

pub use distro::{Distro, Family, PackageManagerKind, RootFilesystem};
pub use system_facts::SystemFacts;
