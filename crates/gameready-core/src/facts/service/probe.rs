//! Reading the system's facts through the command runner.

use std::path::Path;

use crate::exec::{Cmd, CommandRunner};
use crate::facts::domain::SystemFacts;
use crate::facts::errors::FactsError;
use crate::facts::service::os_release::{parse as parse_os_release, OS_RELEASE};
use crate::improvement::KernelVersion;

/// Probes the running system.
///
/// Everything goes through the runner rather than `std::process` directly, so a
/// test can present a Fedora machine with an NVIDIA card to code running on an
/// Ubuntu laptop.
pub fn probe(runner: &dyn CommandRunner) -> Result<SystemFacts, FactsError> {
    let release = runner
        .run(&Cmd::user("uname").arg("-r"))
        .map_err(|source| FactsError::Probe {
            what: "kernel release",
            source,
        })?
        .stdout_trimmed()
        .to_owned();

    let os_release = runner
        .read_to_string(Path::new(OS_RELEASE))
        .map_err(|source| FactsError::Probe {
            what: "os-release",
            source,
        })?;

    let kernel = parse_kernel_release(&release)?;
    Ok(SystemFacts::new(
        parse_os_release(&os_release)?,
        kernel,
        release,
    ))
}

/// Parses the numeric prefix of a `uname -r` string.
///
/// Distro kernels carry suffixes the version itself does not own:
/// `7.0.0-29-generic` on Ubuntu, `6.14.4-arch1-1` on Arch,
/// `6.13.8-200.fc41.x86_64` on Fedora. Only the leading `major.minor.patch`
/// is comparable, so everything from the first `-` is dropped.
///
/// A missing patch level is treated as zero: `6.12` and `6.12.0` are the same
/// kernel for the purposes of a minimum-version check.
///
/// # Examples
///
/// ```
/// use gameready_core::facts::parse_kernel_release;
///
/// let version = parse_kernel_release("7.0.0-29-generic").expect("parses");
/// assert_eq!(version.major, 7);
/// assert_eq!(version.minor, 0);
/// ```
pub fn parse_kernel_release(release: &str) -> Result<KernelVersion, FactsError> {
    let numeric = release.split(['-', '+']).next().unwrap_or(release).trim();

    if numeric.is_empty() {
        return Err(FactsError::KernelRelease {
            release: release.to_owned(),
        });
    }

    let mut parts = numeric.split('.').map(|part| part.parse::<u32>().ok());

    let major = parts
        .next()
        .flatten()
        .ok_or_else(|| FactsError::KernelRelease {
            release: release.to_owned(),
        })?;
    let minor = parts.next().flatten().unwrap_or(0);
    let patch = parts.next().flatten().unwrap_or(0);

    Ok(KernelVersion::new(major, minor, patch))
}

#[cfg(test)]
#[path = "probe_test.rs"]
mod probe_test;
