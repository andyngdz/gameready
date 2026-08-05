//! Reading the system's facts through the command runner.

pub(crate) mod os_release;
mod probe;

pub use os_release::{OS_RELEASE, parse as parse_os_release};
pub use probe::{parse_kernel_release, probe};
