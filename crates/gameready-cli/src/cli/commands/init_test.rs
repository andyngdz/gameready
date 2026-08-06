use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::StatePaths;
use gameready_core::run::Mode;
use tempfile::TempDir;

use super::{InitRequest, run};
use crate::cli::ui::Picker;

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
fn a_dry_run_changes_nothing() {
    let state = TempDir::new().expect("temp dir");
    let games = TempDir::new().expect("temp dir");
    let runner = machine();

    let request = InitRequest {
        games_dir: games.path(),
        mode: Mode::DryRun,
        picker: Picker::TakeAll,
        overlay: None,
    };
    let (report, _) = run(
        &runner,
        StatePaths::new(state.path().to_path_buf()),
        &request,
        &|| Ok(()),
    )
    .expect("init runs");

    assert_ne!(report.status(), gameready_core::run::RunStatus::StepFailed);
    assert!(
        runner.file("/etc/sysctl.d/99-gameready.conf").is_none(),
        "a dry run wrote a file"
    );
}

#[test]
fn the_run_is_reported_with_its_journal() {
    let state = TempDir::new().expect("temp dir");
    let games = TempDir::new().expect("temp dir");

    let request = InitRequest {
        games_dir: games.path(),
        mode: Mode::DryRun,
        picker: Picker::TakeAll,
        overlay: None,
    };
    let (_, text) = run(
        &machine(),
        StatePaths::new(state.path().to_path_buf()),
        &request,
        &|| Ok(()),
    )
    .expect("init runs");

    assert!(text.contains("Journal"), "{text}");
}

#[test]
fn a_scripted_run_does_not_add_mangohud_to_launch_options() {
    let state = TempDir::new().expect("temp dir");
    let games = TempDir::new().expect("temp dir");

    let request = InitRequest {
        games_dir: games.path(),
        mode: Mode::DryRun,
        picker: Picker::TakeAll,
        overlay: None,
    };
    let (_, text) = run(
        &machine(),
        StatePaths::new(state.path().to_path_buf()),
        &request,
        &|| Ok(()),
    )
    .expect("init runs");

    assert!(!text.contains("mangohud %command%"), "{text}");
}

#[test]
fn the_overlay_flag_completes_without_prompting() {
    let state = TempDir::new().expect("temp dir");
    let games = TempDir::new().expect("temp dir");

    let request = InitRequest {
        games_dir: games.path(),
        mode: Mode::DryRun,
        picker: Picker::TakeAll,
        overlay: Some(gameready_core::steam::Overlay::Show),
    };
    let (report, _) = run(
        &machine(),
        StatePaths::new(state.path().to_path_buf()),
        &request,
        &|| Ok(()),
    )
    .expect("init runs");

    assert_ne!(report.status(), gameready_core::run::RunStatus::StepFailed);
}
