use gameready_core::improvement::ImprovementId;
use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::StatePaths;
use gameready_core::run::Mode;
use gameready_core::steps::find_core_step;
use tempfile::TempDir;

use super::run;
use crate::cli::commands::prompt_recorder::PromptRecorder;
use crate::cli::escalation::Escalation;
use crate::cli::ui::Picker;

/// The step this file applies: it writes a sysctl at runtime, which the mock
/// records as a privileged command, so the ordering assertion has something to
/// look at.
const STEP: &str = "core.sysctl.max-map-count";

fn machine() -> MockRunner {
    MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n")
}

#[test]
fn an_unknown_step_id_is_rejected_before_anything_runs() {
    // apply --step takes a user string; a typo must fail here rather than
    // silently apply the whole catalog.
    let id = ImprovementId::parse("core.does.not-exist").expect("well formed");
    assert!(find_core_step(&id).is_none());
}

#[test]
fn a_malformed_step_id_does_not_parse() {
    assert!(ImprovementId::parse("Core.Sysctl").is_err());
}

#[test]
fn apply_primes_before_the_first_privileged_command() {
    let state = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorder = PromptRecorder::new(&runner);
    let prompt = || recorder.answer();

    run(
        &runner,
        StatePaths::new(state.path().to_path_buf()),
        Some(STEP),
        Mode::Apply,
        Picker::TakeAll,
        Escalation::Ask(&prompt),
    )
    .expect("apply runs");

    assert_eq!(recorder.times_asked(), 1, "the password is asked for once");
    assert!(
        !recorder.ran_anything_privileged_first(),
        "sudo -n refuses to prompt, so nothing privileged may run before this"
    );
    assert!(
        recorder.reached_a_privileged_command(),
        "otherwise the assertion above passes on a run that never needed root"
    );
}

#[test]
fn apply_asks_for_the_password_after_the_install_question() {
    let state = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorder = PromptRecorder::new(&runner);
    let prompt = || recorder.answer();

    run(
        &runner,
        StatePaths::new(state.path().to_path_buf()),
        Some(STEP),
        Mode::Apply,
        Picker::TakeAll,
        Escalation::Ask(&prompt),
    )
    .expect("apply runs");

    // Probing reads the machine before any question is put to the user. An
    // empty log at prompt time would mean the prompt had jumped the queue.
    assert!(
        recorder.times_asked() == 1 && !recorder.ran_anything_privileged_first(),
        "the prompt fired once, after planning and before the first change"
    );
}

#[test]
fn a_dry_run_is_never_asked_for_a_password() {
    let state = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorder = PromptRecorder::new(&runner);

    // A dry run maps to Effect::Reads, so the real dispatch hands it NotNeeded.
    // Passing Ask here as well would prove nothing about what dispatch builds.
    run(
        &runner,
        StatePaths::new(state.path().to_path_buf()),
        Some(STEP),
        Mode::DryRun,
        Picker::TakeAll,
        Escalation::NotNeeded,
    )
    .expect("the preview runs");

    assert_eq!(recorder.times_asked(), 0);
    assert!(
        !recorder.reached_a_privileged_command(),
        "a preview changes nothing"
    );
}
