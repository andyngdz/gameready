//! Parsing a Proton-GE GitHub release into the parts the step needs.

use serde::Deserialize;

/// What the step needs from one GitHub release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtonRelease {
    /// The release tag, which is also the directory name after extraction.
    pub tag: String,
    /// Download URL for the x86_64 tarball.
    pub tarball_url: String,
    /// Download URL for the x86_64 sha512sum file.
    pub checksum_url: String,
}

/// Parses the GitHub API JSON for `/repos/.../releases/latest` into a
/// [`ProtonRelease`], picking the x86_64 assets and skipping aarch64.
///
/// Returns `None` when the response is missing a tag or the expected assets.
#[must_use]
pub fn parse_release(json: &str) -> Option<ProtonRelease> {
    let release: GitHubRelease = serde_json::from_str(json).ok()?;
    let tag = release.tag_name;

    let tarball_url = release
        .assets
        .iter()
        .find(|asset| is_x86_tarball(&asset.name))
        .map(|asset| asset.browser_download_url.clone())?;

    let checksum_url = release
        .assets
        .iter()
        .find(|asset| is_x86_checksum(&asset.name))
        .map(|asset| asset.browser_download_url.clone())?;

    Some(ProtonRelease {
        tag,
        tarball_url,
        checksum_url,
    })
}

/// Extracts the expected hex hash from a sha512sum file.
///
/// The format is `<hex-hash>  <filename>` (two spaces between hash and name).
/// Only the first line that contains the tarball name is used.
#[must_use]
pub fn parse_checksum(checksum_text: &str, tarball_name: &str) -> Option<String> {
    checksum_text
        .lines()
        .find(|line| line.contains(tarball_name))
        .and_then(|line| line.split_whitespace().next())
        .map(str::to_owned)
}

/// The tarball filename for a tag, e.g. `GE-Proton11-3.tar.gz`.
#[must_use]
pub fn tarball_name(tag: &str) -> String {
    format!("{tag}.tar.gz")
}

const AARCH64: &str = "aarch64";

fn is_x86_tarball(name: &str) -> bool {
    name.ends_with(".tar.gz") && !name.contains(AARCH64)
}

fn is_x86_checksum(name: &str) -> bool {
    name.ends_with(".sha512sum") && !name.contains(AARCH64)
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[cfg(test)]
#[path = "proton_ge_test.rs"]
mod proton_ge_test;
