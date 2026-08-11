//! Parsing a Proton-GE GitHub release into the parts the step needs.

use serde::Deserialize;

/// What the step needs from one GitHub release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtonRelease {
    /// The release tag, which is also the directory name after extraction.
    pub tag: String,

    /// The x86_64 tarball's filename, as the release itself names it.
    ///
    /// Read from the release rather than built from the tag. Upstream renamed
    /// the asset to `<tag>-x86_64.tar.gz` at GE-Proton11-4, and a name guessed
    /// from the tag matches no line in the checksum file, so every run since
    /// would fail before it downloaded anything.
    pub tarball_name: String,

    /// Download URL for the x86_64 tarball.
    pub tarball_url: String,
    /// Download URL for the x86_64 sha512sum file.
    pub checksum_url: String,

    /// How big the tarball is, in bytes.
    ///
    /// Read from the release rather than from a HEAD request, so the download
    /// knows what it is aiming at before it opens a connection. Zero when the
    /// API did not report it, which the caller reads as "no total to show"
    /// rather than as an empty file.
    pub tarball_bytes: u64,
}

impl ProtonRelease {
    /// The directory the tarball extracts to, which is also the name Steam
    /// registers the tool under.
    ///
    /// The tarball's top-level directory is its own filename without the
    /// extension, and upstream's manifest names the tool the same thing. That
    /// stopped being the tag at GE-Proton11-4, where the asset became
    /// `<tag>-x86_64.tar.gz` and the extracted directory followed it.
    #[must_use]
    pub fn install_name(&self) -> &str {
        self.tarball_name
            .strip_suffix(TARBALL_SUFFIX)
            .unwrap_or(&self.tarball_name)
    }
}

/// What every release tarball is compressed as.
const TARBALL_SUFFIX: &str = ".tar.gz";

/// Parses the GitHub API JSON for `/repos/.../releases/latest` into a
/// [`ProtonRelease`], picking the x86_64 assets and skipping aarch64.
///
/// Returns `None` when the response is missing a tag or the expected assets.
#[must_use]
pub fn parse_release(json: &str) -> Option<ProtonRelease> {
    let release: GitHubRelease = serde_json::from_str(json).ok()?;
    let tag = release.tag_name;

    let tarball = release
        .assets
        .iter()
        .find(|asset| is_x86_tarball(&asset.name))?;

    let checksum_url = release
        .assets
        .iter()
        .find(|asset| is_x86_checksum(&asset.name))
        .map(|asset| asset.browser_download_url.clone())?;

    Some(ProtonRelease {
        tag,
        tarball_name: tarball.name.clone(),
        tarball_url: tarball.browser_download_url.clone(),
        checksum_url,
        tarball_bytes: tarball.size,
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

/// The newest GE-Proton build among a set of installed tool names.
///
/// Compared on the two numbers in the tag rather than as text, which would rank
/// `GE-Proton9-20` above `GE-Proton11-3`. Anything that is not a GE-Proton tag
/// is ignored, so a directory holding some other compatibility tool cannot win.
#[must_use]
pub fn newest_ge_proton(installed: &[String]) -> Option<&str> {
    installed
        .iter()
        .filter_map(|name| ge_version(name).map(|version| (version, name.as_str())))
        .max_by_key(|(version, _)| *version)
        .map(|(_, name)| name)
}

/// The release and revision numbers in a GE-Proton tag, when it is one.
fn ge_version(name: &str) -> Option<(u32, u32)> {
    let rest = name.strip_prefix(GE_PROTON_PREFIX)?;
    // A tag has always carried both numbers so far. One that does not is read
    // as revision zero rather than dropped, so a future naming change costs the
    // build its place in the order instead of its place in the list.
    let (release, revision) = rest.split_once('-').unwrap_or((rest, "0"));
    // Since GE-Proton11-4 the directory carries an architecture suffix, so the
    // revision runs only as far as its digits. Reading "5-x86_64" whole drops
    // the newest build out of the ranking entirely.
    let revision = revision.split(|c: char| !c.is_ascii_digit()).next()?;
    Some((release.parse().ok()?, revision.parse().ok()?))
}

/// What every Proton-GE tag starts with.
const GE_PROTON_PREFIX: &str = "GE-Proton";

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

    /// How big the asset is, which GitHub reports and the download would
    /// otherwise have to discover by finishing.
    #[serde(default)]
    size: u64,
}

#[cfg(test)]
#[path = "proton_ge_test.rs"]
mod proton_ge_test;
