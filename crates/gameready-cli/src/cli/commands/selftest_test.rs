use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::StatePaths;
use tempfile::TempDir;

use super::run;

#[test]
fn a_step_that_applies_and_reverts_cleanly_passes() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");

    let (status, text) =
        run(&runner, StatePaths::new(dir.path().to_path_buf()), None).expect("selftest runs");

    // The mock never moves the runtime value, so verification after apply
    // fails and the selftest must say so rather than report a pass.
    assert_eq!(status, gameready_core::run::RunStatus::StepFailed);
    assert!(text.contains("core.sysctl.max-map-count"), "{text}");
    assert!(text.contains("verify failed"), "{text}");
}

#[test]
fn selftest_leaves_nothing_behind() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");

    let _ = run(&runner, StatePaths::new(dir.path().to_path_buf()), None).expect("selftest runs");

    assert!(
        runner.file("/etc/sysctl.d/99-gameready.conf").is_none(),
        "selftest applied a change and did not roll it back"
    );
}

#[test]
fn a_step_filter_runs_only_that_step() {
    // The bug this guards against: --step was parsed and then ignored, so every
    // step ran regardless.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");

    let (_, text) = run(
        &runner,
        StatePaths::new(dir.path().to_path_buf()),
        Some("core.io.scheduler"),
    )
    .expect("selftest runs");

    assert!(text.contains("core.io.scheduler"), "{text}");
    assert!(!text.contains("core.sysctl.max-map-count"), "{text}");
}
