use std::path::Path;

use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::StatePaths;
use tempfile::TempDir;

use super::run;
use crate::cli::commands::prompt_recorder::PromptRecorder;
use crate::cli::escalation::Escalation;

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

    let (status, text) = run(
        &runner,
        StatePaths::new(dir.path().to_path_buf()),
        None,
        Path::new("/nonexistent/gameready-test/games"),
        Escalation::NotNeeded,
    )
    .expect("selftest runs");

    // The mock never moves the runtime value, so verification after apply
    // fails and the selftest must say so rather than report a pass.
    assert_eq!(status, gameready_core::run::RunStatus::StepFailed);
    assert!(text.contains("vm.max_map_count"), "{text}");
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

    let _ = run(
        &runner,
        StatePaths::new(dir.path().to_path_buf()),
        None,
        Path::new("/nonexistent/gameready-test/games"),
        Escalation::NotNeeded,
    )
    .expect("selftest runs");

    assert!(
        runner.file("/etc/sysctl.d/99-gameready.conf").is_none(),
        "selftest applied a change and did not roll it back"
    );
}

#[test]
fn selftest_primes_before_the_first_privileged_command() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");
    let recorder = PromptRecorder::new(&runner);
    let prompt = || recorder.answer();

    run(
        &runner,
        StatePaths::new(dir.path().to_path_buf()),
        Some("core.sysctl.max-map-count"),
        Path::new("/nonexistent/gameready-test/games"),
        Escalation::Ask(&prompt),
    )
    .expect("selftest runs");

    assert_eq!(recorder.times_asked(), 1, "the password is asked for once");
    assert!(
        !recorder.ran_anything_privileged_first(),
        "selftest applies for real, so the cache has to be warm before it starts"
    );
    assert!(
        recorder.reached_a_privileged_command(),
        "otherwise the assertion above passes on a run that never needed root"
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
        Path::new("/nonexistent/gameready-test/games"),
        Escalation::NotNeeded,
    )
    .expect("selftest runs");

    assert!(text.contains("I/O schedulers"), "{text}");
    assert!(!text.contains("vm.max_map_count"), "{text}");
}
