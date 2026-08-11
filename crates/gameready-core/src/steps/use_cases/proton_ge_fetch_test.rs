use indoc::indoc;

use crate::infra::exec::MockRunner;
use crate::steps::constants::PROTON_GE_LATEST_URL;

use super::*;

fn release_json() -> &'static str {
    indoc! {r#"
        {
          "tag_name": "GE-Proton11-5",
          "assets": [
            {
              "name": "GE-Proton11-5-x86_64.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz"
            },
            {
              "name": "GE-Proton11-5-x86_64.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-x86_64.sha512sum"
            }
          ]
        }
    "#}
}

#[test]
fn fetch_release_parses_github_api_response() {
    let runner = MockRunner::new()
        .with_binary("curl")
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), release_json());

    let release = fetch_release(&runner).expect("should parse");
    assert_eq!(release.tag, "GE-Proton11-5");
    assert_eq!(
        release.tarball_url,
        "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz"
    );
}

#[test]
fn the_checksum_is_looked_up_under_the_name_the_release_gives_the_tarball() {
    // The tarball is not named after the tag alone since GE-Proton11-4, so a
    // name built from the tag finds no line in the checksum file and the run
    // dies before it downloads anything.
    let tarball = "GE-Proton11-5-x86_64.tar.gz";
    let temp = std::env::temp_dir().join(tarball);
    let runner = MockRunner::new()
        .with_binary("curl")
        .with_binary("sha512sum")
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), release_json())
        .answering(
            "curl -sfL https://github.com/dl/GE-Proton11-5-x86_64.sha512sum",
            format!("abc123  {tarball}\n"),
        )
        .answering(
            format!("sha512sum {}", temp.display()),
            format!("abc123  {tarball}\n"),
        )
        .serving(
            "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz",
            "a tarball, as far as this test is concerned",
        );
    let release = fetch_release(&runner).expect("should parse");

    let downloaded = download_verified(&runner, &release, &|_done| {}).expect("verifies");

    assert_eq!(downloaded, temp);
}

#[test]
fn fetch_release_returns_error_on_bad_json() {
    let runner = MockRunner::new()
        .with_binary("curl")
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), "not json");

    assert!(fetch_release(&runner).is_err());
}
