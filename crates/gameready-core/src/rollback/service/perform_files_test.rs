use crate::improvement::Privilege;
use crate::infra::exec::MockRunner;
use crate::journal::digest;

use super::*;

const DROPIN: &str = "/etc/sysctl.d/99-gameready.conf";
const WROTE: &str = "vm.max_map_count = 2147483642\n";

#[test]
fn a_file_still_holding_what_we_wrote_is_deleted() {
    let runner = MockRunner::new().with_file(DROPIN, WROTE);
    let outcome = delete_file(&runner, Path::new(DROPIN), &digest(WROTE), Privilege::Root);

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert!(runner.file(DROPIN).is_none());
}

#[test]
fn a_file_the_user_edited_is_left_where_it_is() {
    // Leaving a stale drop-in is recoverable; clobbering a hand edit is not.
    let runner = MockRunner::new().with_file(DROPIN, "vm.max_map_count = 999\n");
    let outcome = delete_file(&runner, Path::new(DROPIN), &digest(WROTE), Privilege::Root);

    assert!(matches!(outcome, UndoOutcome::Refused { .. }));
    assert!(runner.file(DROPIN).is_some(), "the edited file was deleted");
}

#[test]
fn a_file_already_gone_needs_no_undo() {
    let runner = MockRunner::new();
    let outcome = delete_file(&runner, Path::new(DROPIN), &digest(WROTE), Privilege::Root);

    assert!(matches!(outcome, UndoOutcome::AlreadyGone));
}

#[test]
fn a_pre_image_is_copied_back_over_the_file_it_came_from() {
    let backup = "/state/backups/01/localconfig.vdf";
    let runner = MockRunner::new()
        .with_file(backup, "original\n")
        .with_file(DROPIN, "replaced\n");

    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new(backup),
        Privilege::User,
    );

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert_eq!(runner.file(DROPIN).as_deref(), Some("original\n"));
}

#[test]
fn a_missing_pre_image_fails_rather_than_reporting_a_revert() {
    // Claiming the file was restored from a backup that is not there would tell
    // the user their machine is back to normal when it is not.
    let runner = MockRunner::new().with_file(DROPIN, "replaced\n");
    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new("/state/backups/gone"),
        Privilege::User,
    );

    assert!(matches!(outcome, UndoOutcome::Failed { .. }));
}

#[test]
fn a_directory_tree_that_is_already_gone_needs_no_undo() {
    let runner = MockRunner::new();
    let outcome = remove_dir_tree(
        &runner,
        Path::new("/home/u/.steam/compatibilitytools.d/GE-Proton"),
        Privilege::User,
    );

    assert!(matches!(outcome, UndoOutcome::AlreadyGone));
    assert!(runner.commands().is_empty(), "{:?}", runner.commands());
}

#[test]
fn a_removed_directory_is_named_in_what_the_undo_reports() {
    // The reason a directory undo says which path it took is that the summary
    // is the only place a user sees it; nothing else prints the path.
    let runner = MockRunner::new().with_file("/etc/scx_loader", String::new());
    let outcome = remove_dir(&runner, Path::new("/etc/scx_loader"), Privilege::Root);

    match outcome {
        UndoOutcome::Reverted { detail } => assert!(detail.contains("/etc/scx_loader"), "{detail}"),
        other @ (UndoOutcome::AlreadyGone
        | UndoOutcome::Left { .. }
        | UndoOutcome::Refused { .. }
        | UndoOutcome::Failed { .. }) => panic!("expected a removal, got {other:?}"),
    }
}

#[test]
fn removing_a_directory_uses_rmdir_not_a_file_delete() {
    // Regression: this called remove_file, which is fs::remove_file for a user
    // path and `rm -f` for a root one. Neither can remove a directory at all,
    // so every directory undo reported "was not empty" and left it behind.
    let runner = MockRunner::new().with_file("/home/someone/.config/environment.d", String::new());
    let outcome = remove_dir(
        &runner,
        Path::new("/home/someone/.config/environment.d"),
        Privilege::User,
    );

    assert!(
        matches!(outcome, UndoOutcome::Reverted { .. }),
        "{outcome:?}"
    );
    assert!(
        runner
            .commands()
            .iter()
            .any(|cmd| cmd == "rmdir /home/someone/.config/environment.d"),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn a_users_own_directory_is_removed_without_sudo() {
    // Asking for a password to take away a directory in their own home teaches
    // them to type it without reading what asked.
    let runner = MockRunner::new().with_file("/home/someone/.config/environment.d", String::new());
    let _ = remove_dir(
        &runner,
        Path::new("/home/someone/.config/environment.d"),
        Privilege::User,
    );

    assert!(
        !runner.commands().iter().any(|cmd| cmd.starts_with("sudo")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn a_directory_something_else_still_uses_is_left_alone() {
    let runner = MockRunner::new()
        .with_file("/home/someone/.config/environment.d", String::new())
        .failing("rmdir /home/someone/.config/environment.d");
    let outcome = remove_dir(
        &runner,
        Path::new("/home/someone/.config/environment.d"),
        Privilege::User,
    );

    match outcome {
        UndoOutcome::Left { reason } => assert!(reason.contains("was not empty"), "{reason}"),
        other @ (UndoOutcome::Reverted { .. }
        | UndoOutcome::AlreadyGone
        | UndoOutcome::Refused { .. }
        | UndoOutcome::Failed { .. }) => panic!("expected it to be left, got {other:?}"),
    }
}
