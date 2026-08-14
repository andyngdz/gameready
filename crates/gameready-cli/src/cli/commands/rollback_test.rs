use gameready_core::improvement::ImprovementId;
use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::{Change, Journal, JournalEvent, RunId, StatePaths};
use tempfile::TempDir;

use super::run;
use crate::cli::commands::prompt_recorder::PromptRecorder;
use crate::cli::escalation::Escalation;

/// Writes a run whose one change is undone by a privileged command, so the
/// ordering assertion has a `sudo ` line to look for.
fn recorded_sysctl_run(dir: &TempDir) -> StatePaths {
    let paths = StatePaths::new(dir.path().to_path_buf());
    let mut journal = Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
    journal
        .append(JournalEvent::Changed {
            step: ImprovementId::from_static("core.sysctl.max-map-count"),
            change: Change::SysctlRuntime {
                key: "vm.max_map_count".to_owned(),
                previous: "1048576".to_owned(),
            },
        })
        .expect("appends");
    paths
}

#[test]
fn an_empty_journal_is_reported_rather_than_treated_as_an_error_state() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let paths = StatePaths::new(dir.path().to_path_buf());

    let error =
        run(&runner, paths, None, Escalation::NotNeeded).expect_err("there is nothing to undo");

    assert!(error.to_string().contains("no runs"), "{error}");
}

#[test]
fn a_malformed_run_id_is_rejected_before_anything_runs() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let paths = StatePaths::new(dir.path().to_path_buf());

    let error = run(&runner, paths, Some("not-a-ulid"), Escalation::NotNeeded).expect_err("bad id");

    assert!(error.to_string().contains("not a run id"), "{error}");
    assert!(runner.commands().is_empty());
}

#[test]
fn rollback_primes_before_the_first_privileged_command() {
    let dir = TempDir::new().expect("temp dir");
    let paths = recorded_sysctl_run(&dir);
    let runner = MockRunner::new();
    let recorder = PromptRecorder::new(&runner);
    let prompt = || recorder.answer();

    run(&runner, paths, None, Escalation::Ask(&prompt)).expect("rollback runs");

    assert_eq!(recorder.times_asked(), 1, "the password is asked for once");
    assert!(
        !recorder.ran_anything_privileged_first(),
        "the undo runs as root, so the cache has to be warm before it starts"
    );
    assert!(
        recorder.reached_a_privileged_command(),
        "otherwise the assertion above passes on a run that never needed root"
    );
}

#[test]
fn a_run_with_nothing_to_undo_does_not_ask_for_a_password() {
    let dir = TempDir::new().expect("temp dir");
    let paths = StatePaths::new(dir.path().to_path_buf());
    let run_id = RunId::generate();
    let mut journal = Journal::open(paths.clone(), run_id).expect("journal opens");
    journal
        .append(JournalEvent::RunBegin {
            argv: vec!["gameready".to_owned(), "apply".to_owned()],
            tool_version: "test".to_owned(),
        })
        .expect("appends");

    let runner = MockRunner::new();
    let recorder = PromptRecorder::new(&runner);
    let prompt = || recorder.answer();

    let (_, text) = run(
        &runner,
        paths,
        Some(&run_id.to_string()),
        Escalation::Ask(&prompt),
    )
    .expect("the run resolves");

    assert!(text.contains("no changes to undo"), "{text}");
    assert_eq!(
        recorder.times_asked(),
        0,
        "asking for a password to undo nothing is a question with no purpose"
    );
}

/// Writes a run whose one change is a file in the user's own home, so undoing
/// it is a user's job.
fn recorded_user_file_run(dir: &TempDir, file: &std::path::Path) -> StatePaths {
    let paths = StatePaths::new(dir.path().to_path_buf());
    let mut journal = Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
    journal
        .append(JournalEvent::Changed {
            step: ImprovementId::from_static("core.sysctl.max-map-count"),
            change: Change::FileWritten {
                path: file.to_path_buf(),
                existed: false,
                backup: None,
                sha256_after: gameready_core::journal::digest("wrote this"),
                mode: 0o644,
                privilege: gameready_core::improvement::Privilege::User,
            },
        })
        .expect("appends");
    paths
}

#[test]
fn undoing_a_run_that_only_touched_the_users_files_never_asks_for_a_password() {
    // Asking to delete a file they own teaches a user to type their password
    // without reading what asked for it.
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("99-gameready.conf");
    std::fs::write(&file, "wrote this").expect("the file the run wrote");
    let paths = recorded_user_file_run(&dir, &file);
    let runner = MockRunner::new();
    let asked = std::cell::Cell::new(false);
    let prompt = || {
        asked.set(true);
        Ok(())
    };

    run(&runner, paths, None, Escalation::Ask(&prompt)).expect("the rollback runs");

    assert!(!asked.get(), "a user-owned file was undone behind a prompt");
}
