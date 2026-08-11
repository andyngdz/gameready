use indoc::indoc;

use super::*;

#[test]
fn parses_a_release_with_x86_and_aarch64_assets() {
    // Asset order and naming as GitHub serves them since GE-Proton11-4:
    // aarch64 first, and the x86_64 tarball no longer named after the tag
    // alone. A name built from the tag matches nothing in the checksum file.
    let json = indoc! {r#"
        {
          "tag_name": "GE-Proton11-5",
          "assets": [
            {
              "name": "GE-Proton11-5-aarch64.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-aarch64.sha512sum"
            },
            {
              "name": "GE-Proton11-5-aarch64.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-aarch64.tar.gz"
            },
            {
              "name": "GE-Proton11-5-x86_64.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-x86_64.sha512sum",
              "size": 158
            },
            {
              "name": "GE-Proton11-5-x86_64.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz",
              "size": 533066604
            }
          ]
        }
    "#};

    let release = parse_release(json).expect("should parse");
    assert_eq!(release.tag, "GE-Proton11-5");
    assert_eq!(release.tarball_name, "GE-Proton11-5-x86_64.tar.gz");
    assert_eq!(
        release.tarball_url,
        "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz"
    );
    assert_eq!(
        release.checksum_url,
        "https://github.com/dl/GE-Proton11-5-x86_64.sha512sum"
    );
    // The tarball's size, not the checksum file's: the progress bar is about
    // the 508 MB, not the 158 bytes beside it.
    assert_eq!(release.tarball_bytes, 533_066_604);
}

#[test]
fn a_release_that_reports_no_size_parses_with_nothing_to_aim_at() {
    // Every release GitHub serves carries one. A run must not fall over
    // because a mirror or a fixture left the field out.
    let json = indoc! {r#"
        {
          "tag_name": "GE-Proton11-3",
          "assets": [
            {
              "name": "GE-Proton11-3.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3.tar.gz"
            },
            {
              "name": "GE-Proton11-3.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3.sha512sum"
            }
          ]
        }
    "#};

    let release = parse_release(json).expect("should parse");
    assert_eq!(release.tarball_bytes, 0);
}

#[test]
fn returns_none_for_missing_tarball_asset() {
    let json = indoc! {r#"
        {
          "tag_name": "GE-Proton11-3",
          "assets": [
            {
              "name": "GE-Proton11-3.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3.sha512sum"
            }
          ]
        }
    "#};

    assert!(parse_release(json).is_none());
}

#[test]
fn returns_none_for_invalid_json() {
    assert!(parse_release("not json").is_none());
}

#[test]
fn parses_checksum_with_two_space_separator() {
    let checksum_text = "abc123def456  GE-Proton11-3.tar.gz\n";
    let hash = parse_checksum(checksum_text, "GE-Proton11-3.tar.gz");
    assert_eq!(hash.as_deref(), Some("abc123def456"));
}

#[test]
fn parses_checksum_ignoring_unrelated_lines() {
    let checksum_text = indoc! {"
        deadbeef  other-file.txt
        abc123def456  GE-Proton11-3.tar.gz
    "};
    let hash = parse_checksum(checksum_text, "GE-Proton11-3.tar.gz");
    assert_eq!(hash.as_deref(), Some("abc123def456"));
}

#[test]
fn returns_none_when_tarball_not_in_checksum() {
    let checksum_text = "abc123  other-file.tar.gz\n";
    assert!(parse_checksum(checksum_text, "GE-Proton11-3.tar.gz").is_none());
}

#[test]
fn the_install_directory_is_the_tarball_name_rather_than_the_tag() {
    // tar lays the archive down under its own filename, and upstream's own
    // manifest registers the tool under that same name.
    let release = ProtonRelease {
        tag: "GE-Proton11-5".to_owned(),
        tarball_name: "GE-Proton11-5-x86_64.tar.gz".to_owned(),
        tarball_url: String::new(),
        checksum_url: String::new(),
        tarball_bytes: 0,
    };

    assert_eq!(release.install_name(), "GE-Proton11-5-x86_64");
}

fn tools(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn the_newest_build_is_picked_by_number_not_by_text() {
    // Sorted as text, GE-Proton9-20 wins, and the user gets a build two major
    // releases old without being told.
    let installed = tools(&["GE-Proton9-20", "GE-Proton11-3", "GE-Proton10-15"]);

    assert_eq!(newest_ge_proton(&installed), Some("GE-Proton11-3"));
}

#[test]
fn the_revision_breaks_a_tie_within_one_release() {
    let installed = tools(&["GE-Proton11-3", "GE-Proton11-20"]);

    assert_eq!(newest_ge_proton(&installed), Some("GE-Proton11-20"));
}

#[test]
fn an_architecture_suffix_does_not_cost_a_build_its_place() {
    // Installs made since GE-Proton11-4 carry the suffix. Reading the revision
    // as "5-x86_64" drops the newest build and pins the game to an older one.
    let installed = tools(&["GE-Proton11-3", "GE-Proton11-5-x86_64"]);

    assert_eq!(newest_ge_proton(&installed), Some("GE-Proton11-5-x86_64"));
}

#[test]
fn some_other_compatibility_tool_never_wins() {
    let installed = tools(&["Proton-EM-10.0-25", "SteamTinkerLaunch", "GE-Proton11-3"]);

    assert_eq!(newest_ge_proton(&installed), Some("GE-Proton11-3"));
}

#[test]
fn a_machine_with_no_ge_proton_has_no_answer() {
    assert_eq!(newest_ge_proton(&tools(&["SteamTinkerLaunch"])), None);
    assert_eq!(newest_ge_proton(&[]), None);
}
