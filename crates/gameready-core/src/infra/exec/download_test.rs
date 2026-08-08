use std::path::Path;

use super::*;

/// A port nothing listens on, so the connection is refused rather than routed.
/// Keeps these tests offline: no name to resolve, no host to reach.
const REFUSED: &str = "http://127.0.0.1:1/GE-Proton11-3.tar.gz";

#[test]
fn a_transfer_that_never_connects_names_the_url_rather_than_a_path() {
    let dest = Path::new("/nonexistent/never-written");

    let error = fetch(REFUSED, dest, &|_| {}).expect_err("nothing is listening");

    match error {
        ExecError::Download { url, .. } => assert_eq!(url, REFUSED),
        other => panic!("expected a download failure, got {other:?}"),
    }
}

#[test]
fn a_transfer_that_never_connects_writes_nothing_and_counts_nothing() {
    let dest = Path::new("/nonexistent/never-written");
    let counted = std::cell::Cell::new(0_u32);

    let _ = fetch(REFUSED, dest, &|_| counted.set(counted.get() + 1));

    assert_eq!(counted.get(), 0);
    assert!(!dest.exists());
}

#[test]
fn a_url_that_is_not_one_fails_rather_than_panicking() {
    let dest = Path::new("/nonexistent/never-written");

    let error = fetch("not a url at all", dest, &|_| {}).expect_err("not a url");

    assert!(matches!(error, ExecError::Download { .. }), "{error:?}");
}
