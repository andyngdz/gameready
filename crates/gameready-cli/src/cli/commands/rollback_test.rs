use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::StatePaths;
use gameready_core::rollback::PackagePolicy;
use tempfile::TempDir;

use super::run;

#[test]
fn an_empty_journal_is_reported_rather_than_treated_as_an_error_state() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let paths = StatePaths::new(dir.path().to_path_buf());

    let error =
        run(&runner, paths, None, PackagePolicy::Keep).expect_err("there is nothing to undo");

    assert!(error.to_string().contains("no runs"), "{error}");
}

#[test]
fn a_malformed_run_id_is_rejected_before_anything_runs() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let paths = StatePaths::new(dir.path().to_path_buf());

    let error = run(&runner, paths, Some("not-a-ulid"), PackagePolicy::Keep).expect_err("bad id");

    assert!(error.to_string().contains("not a run id"), "{error}");
    assert!(runner.commands().is_empty());
}
