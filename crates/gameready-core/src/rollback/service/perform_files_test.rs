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

const BACKUP: &str = "/state/backups/01/localconfig.vdf";
const REPLACED: &str = "replaced\n";

#[test]
fn a_pre_image_is_copied_back_over_the_file_it_came_from() {
    let runner = MockRunner::new()
        .with_file(BACKUP, "original\n")
        .with_file(DROPIN, REPLACED);

    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new(BACKUP),
        Some(&digest(REPLACED)),
        Privilege::User,
    );

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert_eq!(runner.file(DROPIN).as_deref(), Some("original\n"));
}

#[test]
fn a_missing_pre_image_fails_rather_than_reporting_a_revert() {
    // Claiming the file was restored from a backup that is not there would tell
    // the user their machine is back to normal when it is not.
    let runner = MockRunner::new().with_file(DROPIN, REPLACED);
    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new("/state/backups/gone"),
        Some(&digest(REPLACED)),
        Privilege::User,
    );

    assert!(matches!(outcome, UndoOutcome::Failed { .. }));
}

#[test]
fn a_file_edited_after_we_replaced_it_is_not_overwritten_by_the_pre_image() {
    // The in-place case is the one that matters most: the file held the user's
    // own content before gameready touched it, so the pre-image is not a
    // superset of what a hand edit would destroy.
    let runner = MockRunner::new()
        .with_file(BACKUP, "original\n")
        .with_file(DROPIN, "the user's own edit\n");

    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new(BACKUP),
        Some(&digest(REPLACED)),
        Privilege::User,
    );

    assert!(
        matches!(outcome, UndoOutcome::Refused { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        runner.file(DROPIN).as_deref(),
        Some("the user's own edit\n"),
        "the hand edit was clobbered"
    );
}

#[test]
fn a_file_recreated_after_we_removed_it_is_left_alone() {
    // The removal case records no digest, because gameready wrote no bytes it
    // could recognise. Anything at that path now arrived after the run.
    let runner = MockRunner::new()
        .with_file(BACKUP, "original\n")
        .with_file(DROPIN, "something else put this here\n");

    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new(BACKUP),
        None,
        Privilege::User,
    );

    assert!(
        matches!(outcome, UndoOutcome::Refused { .. }),
        "{outcome:?}"
    );
    assert_eq!(
        runner.file(DROPIN).as_deref(),
        Some("something else put this here\n")
    );
}

#[test]
fn restoring_a_file_already_put_back_is_not_read_as_a_hand_edit() {
    // Rollback has to be safe to re-run. After the first restore the file holds
    // the pre-image, which matches neither what the run wrote nor a hand edit.
    let runner = MockRunner::new()
        .with_file(BACKUP, "original\n")
        .with_file(DROPIN, "original\n");

    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new(BACKUP),
        Some(&digest(REPLACED)),
        Privilege::User,
    );

    assert!(matches!(outcome, UndoOutcome::AlreadyGone), "{outcome:?}");
}

#[test]
fn a_pre_image_lands_on_a_path_whose_file_is_gone() {
    // The removal case in its normal shape: gameready deleted the file, nothing
    // recreated it, and the pre-image goes straight back.
    let runner = MockRunner::new().with_file(BACKUP, "original\n");

    let outcome = restore_file(
        &runner,
        Path::new(DROPIN),
        Path::new(BACKUP),
        None,
        Privilege::User,
    );

    assert!(
        matches!(outcome, UndoOutcome::Reverted { .. }),
        "{outcome:?}"
    );
    assert_eq!(runner.file(DROPIN).as_deref(), Some("original\n"));
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
    let runner = MockRunner::new().with_file("/etc/gameready-test", String::new());
    let outcome = remove_dir(&runner, Path::new("/etc/gameready-test"), Privilege::Root);

    match outcome {
        UndoOutcome::Reverted { detail } => {
            assert!(detail.contains("/etc/gameready-test"), "{detail}")
        }
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
