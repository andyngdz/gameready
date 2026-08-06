use indoc::indoc;

use crate::infra::exec::MockRunner;
use crate::steps::constants::PROTON_GE_LATEST_URL;

use super::*;

fn release_json() -> &'static str {
    indoc! {r#"
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
    "#}
}

#[test]
fn fetch_release_parses_github_api_response() {
    let runner = MockRunner::new()
        .with_binary("curl")
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), release_json());

    let release = fetch_release(&runner).expect("should parse");
    assert_eq!(release.tag, "GE-Proton11-3");
    assert_eq!(
        release.tarball_url,
        "https://github.com/dl/GE-Proton11-3.tar.gz"
    );
}

#[test]
fn fetch_release_returns_error_on_bad_json() {
    let runner = MockRunner::new()
        .with_binary("curl")
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), "not json");

    assert!(fetch_release(&runner).is_err());
}
