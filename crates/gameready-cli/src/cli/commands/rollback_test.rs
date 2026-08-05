use gameready_core::journal::StatePaths;
use tempfile::TempDir;

use super::run;

#[test]
fn reports_an_empty_journal_without_failing() {
    let dir = TempDir::new().expect("temp dir");
    let text = run(&StatePaths::new(dir.path().to_path_buf()), None).expect("reads");
    assert!(text.contains("Records   0"));
}

#[test]
fn says_plainly_that_it_changed_nothing() {
    // Until the undo replay lands, this must not read as though it undid work.
    let dir = TempDir::new().expect("temp dir");
    let text = run(&StatePaths::new(dir.path().to_path_buf()), None).expect("reads");
    assert!(text.contains("not implemented"));
    assert!(text.contains("Nothing was changed"));
}
