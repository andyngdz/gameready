//! Reading `/etc/os-release`.

use crate::facts::constants::{ARCH, BAZZITE, DEBIAN, FEDORA, KINOITE, SILVERBLUE};
use crate::facts::domain::{Distro, Family, RootFilesystem};
use crate::facts::errors::FactsError;

/// Where the format is specified to live.
pub const OS_RELEASE: &str = "/etc/os-release";

/// Parses the contents of `/etc/os-release`.
///
/// Hand-rolled rather than pulled from a crate. The format is `KEY=VALUE` with
/// optional quoting, so the parser is short, and the interesting part is not
/// parsing but the family table below, which a crate would hide behind its own
/// idea of what a distro is.
pub fn parse(contents: &str) -> Result<Distro, FactsError> {
    let id = field(contents, "ID").ok_or(FactsError::OsRelease { missing: "ID" })?;
    let id_like = field(contents, "ID_LIKE").unwrap_or_default();
    let name = field(contents, "PRETTY_NAME").unwrap_or_else(|| id.clone());

    let family = family_of(&id, &id_like).ok_or(FactsError::UnsupportedDistro {
        id: id.clone(),
        id_like,
    })?;

    Ok(Distro {
        root_filesystem: root_filesystem_of(&id, contents),
        id,
        name,
        version_id: field(contents, "VERSION_ID"),
        family,
    })
}

/// Reads one key, stripping the quoting the format allows.
fn field(contents: &str, key: &str) -> Option<String> {
    contents
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .map(|value| value.trim().trim_matches(['"', '\'']).to_owned())
        .filter(|value| !value.is_empty())
}

/// Decides which family's conventions apply.
///
/// `ID` is checked before `ID_LIKE` so a distro that names itself explicitly
/// wins. `ID_LIKE` is a space-separated list, and order within it is not
/// meaningful: Pop!_OS ships `ID_LIKE="ubuntu debian"`, so matching on the whole
/// string rather than splitting it is a bug that upstream gameready shipped.
fn family_of(id: &str, id_like: &str) -> Option<Family> {
    if let Some(family) = family_of_token(id) {
        return Some(family);
    }
    id_like.split_whitespace().find_map(family_of_token)
}

/// Maps one `ID`-shaped token to a family.
fn family_of_token(token: &str) -> Option<Family> {
    match token {
        ARCH | "archarm" | "manjaro" | "endeavouros" | "cachyos" | "garuda" => Some(Family::Arch),
        DEBIAN | "ubuntu" | "linuxmint" | "pop" | "zorin" | "elementary" | "raspbian" => {
            Some(Family::Debian)
        }
        BAZZITE | FEDORA | "nobara" | "rhel" | "centos" | "almalinux" | "rocky" => {
            Some(Family::Fedora)
        }
        _ => None,
    }
}

/// Detects an image-based system, where `/usr` is read-only.
///
/// Bazzite and the other Universal Blue images set `VARIANT_ID` to an ostree
/// variant. Steps that install packages or write outside `/etc` must report
/// `NotApplicable` there rather than failing on a permission error.
fn root_filesystem_of(id: &str, contents: &str) -> RootFilesystem {
    let variant = field(contents, "VARIANT_ID").unwrap_or_default();
    let image_based = matches!(id, BAZZITE | SILVERBLUE | KINOITE | "steamos")
        || variant.contains("ostree")
        || variant.contains(SILVERBLUE)
        || variant.contains(KINOITE);

    if image_based {
        RootFilesystem::ImageBased
    } else {
        RootFilesystem::Mutable
    }
}

#[cfg(test)]
#[path = "os_release_test.rs"]
mod os_release_test;
