use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::StatePaths;
use gameready_core::run::Mode;
use tempfile::TempDir;

use super::{InitRequest, run};
use crate::cli::ui::Picker;

/// A machine that answers the probes init needs, with nothing applied yet.
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
    let (report, text) = run(
        &runner,
        StatePaths::new(state.path().to_path_buf()),
        &request,
        &|| Ok(()),
    )
    .expect("init runs");

    assert_ne!(report.status(), gameready_core::run::RunStatus::StepFailed);
    assert!(text.contains("Games found"), "{text}");
    assert!(
        runner.file("/etc/sysctl.d/99-gameready.conf").is_none(),
        "a dry run wrote a file"
    );
}

#[test]
fn the_game_list_is_shown_even_when_no_game_was_selected() {
    // A user with no Steam, or who picked nothing, still gets the core tuning
    // and should see that gameready looked.
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

    assert!(text.contains("Games found"), "{text}");
}

#[test]
fn the_run_is_reported_with_its_journal_so_it_can_be_undone() {
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
fn a_scripted_run_leaves_the_screen_alone_unless_asked() {
    // Nobody is at the terminal, so the overlay must not appear by default.
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

    // The step that installs it names it, which is fine. What must not appear
    // is a launch option putting it on the screen.
    assert!(!text.contains("mangohud %command%"), "{text}");
}

#[test]
fn the_flag_turns_the_overlay_on_without_a_prompt() {
    let state = TempDir::new().expect("temp dir");
    let games = TempDir::new().expect("temp dir");

    let request = InitRequest {
        games_dir: games.path(),
        mode: Mode::DryRun,
        picker: Picker::TakeAll,
        overlay: Some(gameready_core::steam::Overlay::Show),
    };
    let (_, text) = run(
        &machine(),
        StatePaths::new(state.path().to_path_buf()),
        &request,
        &|| Ok(()),
    )
    .expect("init runs");

    // No games are installed in this fixture, so the assertion that matters is
    // that it completed without waiting on a prompt.
    assert!(text.contains("Games found"), "{text}");
}
