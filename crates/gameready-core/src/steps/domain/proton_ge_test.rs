use indoc::indoc;

use super::*;

#[test]
fn parses_a_release_with_x86_and_aarch64_assets() {
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
            },
            {
              "name": "GE-Proton11-3-aarch64.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3-aarch64.tar.gz"
            },
            {
              "name": "GE-Proton11-3-aarch64.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3-aarch64.sha512sum"
            }
          ]
        }
    "#};

    let release = parse_release(json).expect("should parse");
    assert_eq!(release.tag, "GE-Proton11-3");
    assert_eq!(
        release.tarball_url,
        "https://github.com/dl/GE-Proton11-3.tar.gz"
    );
    assert_eq!(
        release.checksum_url,
        "https://github.com/dl/GE-Proton11-3.sha512sum"
    );
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
fn tarball_name_matches_the_tag() {
    assert_eq!(tarball_name("GE-Proton11-3"), "GE-Proton11-3.tar.gz");
}
