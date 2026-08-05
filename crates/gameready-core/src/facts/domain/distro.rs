//! Which Linux distribution this is, and what installs software on it.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::facts::constants::{ARCH, DEBIAN, FEDORA};

/// The distribution family, which is what actually decides behaviour.
///
/// Keyed on family rather than on individual distros because there are far too
/// many derivatives to enumerate and they nearly all inherit their parent's
/// package manager and paths. The exact `ID` is kept alongside for reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Arch, EndeavourOS, Manjaro, CachyOS.
    Arch,
    /// Debian, Ubuntu, Mint, Pop!_OS, Zorin.
    Debian,
    /// Fedora, Nobara, RHEL derivatives.
    Fedora,
}

impl Family {
    /// The package manager this family uses.
    #[must_use]
    pub const fn package_manager(self) -> PackageManagerKind {
        match self {
            Self::Arch => PackageManagerKind::Pacman,
            Self::Debian => PackageManagerKind::Apt,
            Self::Fedora => PackageManagerKind::Dnf,
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Arch => ARCH,
            Self::Debian => DEBIAN,
            Self::Fedora => FEDORA,
        })
    }
}

/// Which tool installs packages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageManagerKind {
    Pacman,
    Apt,
    Dnf,
}

impl PackageManagerKind {
    /// The binary name, which is also what a probe looks for on `PATH`.
    #[must_use]
    pub const fn binary(self) -> &'static str {
        match self {
            Self::Pacman => "pacman",
            Self::Apt => "apt-get",
            Self::Dnf => "dnf",
        }
    }
}

impl fmt::Display for PackageManagerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.binary())
    }
}

/// Whether the root filesystem can be written to at all.
///
/// Image-based systems such as Bazzite and Fedora Silverblue mount `/usr`
/// read-only and layer packages through `rpm-ostree`, so a step that writes
/// there is not merely likely to fail, it is the wrong operation. Detected so
/// those steps report `NotApplicable` with a reason rather than a permission
/// error the user cannot act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RootFilesystem {
    /// A normal mutable system.
    Mutable,
    /// Image-based: `/usr` is read-only and packages are layered.
    ImageBased,
}

/// What `/etc/os-release` said, plus what was derived from it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Distro {
    /// The `ID` field verbatim, such as `nobara` or `pop`.
    pub id: String,

    /// The `PRETTY_NAME` field, for reporting.
    pub name: String,

    /// The `VERSION_ID` field, absent on rolling releases.
    pub version_id: Option<String>,

    /// Which family's conventions apply.
    pub family: Family,

    /// Whether the system is image-based.
    pub root_filesystem: RootFilesystem,
}

impl Distro {
    /// The package manager to drive on this system.
    #[must_use]
    pub const fn package_manager(&self) -> PackageManagerKind {
        self.family.package_manager()
    }
}

#[cfg(test)]
#[path = "distro_test.rs"]
mod distro_test;
