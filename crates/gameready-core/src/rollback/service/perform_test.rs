use crate::improvement::Privilege;
use std::path::PathBuf;

use crate::infra::exec::MockRunner;
use crate::journal::digest;

use super::*;

const DROPIN: &str = "/etc/sysctl.d/99-gameready.conf";
const WROTE: &str = "vm.max_map_count = 2147483642\n";

fn delete_undo(expected: &str) -> Undo {
    Undo::DeleteFile {
        path: PathBuf::from(DROPIN),
        expect_sha256: expected.to_owned(),
        privilege: Privilege::Root,
    }
}

#[test]
fn deletes_a_file_that_is_still_what_we_wrote() {
    let runner = MockRunner::new().with_file(DROPIN, WROTE);
    let outcome = perform(&delete_undo(&digest(WROTE)), &runner);

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert!(runner.file(DROPIN).is_none());
}

#[test]
fn refuses_to_delete_a_file_the_user_edited() {
    // Leaving a stale drop-in is recoverable; clobbering a hand edit is not.
    let runner = MockRunner::new().with_file(DROPIN, "vm.max_map_count = 999\n");
    let outcome = perform(&delete_undo(&digest(WROTE)), &runner);

    assert!(matches!(outcome, UndoOutcome::Refused { .. }));
    assert!(runner.file(DROPIN).is_some(), "the edited file was deleted");
}

#[test]
fn a_file_a_later_run_rewrote_points_at_that_run_instead_of_blaming_the_user() {
    // The managed header carries the run id, so applying the same step twice
    // produces different bytes. Undoing the older run must not report the
    // newer run's file as a user edit.
    let later = indoc::indoc! {"
        # Managed by gameready 0.1.0 - step=core.sysctl.max-map-count run=01KZ8EQST7AB67NWJJ07CTDSJC
        vm.max_map_count = 2147483642
    "};
    let runner = MockRunner::new().with_file(DROPIN, later);

    let outcome = perform(&delete_undo(&digest(WROTE)), &runner);

    match outcome {
        UndoOutcome::Left { reason } => {
            assert!(reason.contains("01KZ8EQST7AB67NWJJ07CTDSJC"), "{reason}");
            assert!(reason.contains("rollback --run"), "{reason}");
        }
        other @ (UndoOutcome::Reverted { .. }
        | UndoOutcome::AlreadyGone
        | UndoOutcome::Refused { .. }
        | UndoOutcome::Failed { .. }) => {
            panic!("expected a pointer to the owning run, got {other:?}")
        }
    }
    assert!(
        runner.file(DROPIN).is_some(),
        "another run's file was deleted"
    );
}

#[test]
fn a_file_already_gone_is_not_a_failure() {
    // Rollback has to be safe to re-run after a partial undo.
    let runner = MockRunner::new();
    let outcome = perform(&delete_undo(&digest(WROTE)), &runner);

    assert_eq!(outcome, UndoOutcome::AlreadyGone);
    assert!(!outcome.is_failure());
}

#[test]
fn restores_a_sysctl_to_its_previous_value() {
    let runner = MockRunner::new();
    let undo = Undo::SetSysctl {
        key: "vm.max_map_count".to_owned(),
        value: "1048576".to_owned(),
    };
    let outcome = perform(&undo, &runner);

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert_eq!(
        runner.commands(),
        ["sudo sysctl -w vm.max_map_count=1048576"]
    );
}

#[test]
fn packages_are_left_installed_by_default() {
    let runner = MockRunner::new();
    let undo = Undo::ReportPackages {
        manager: "apt".to_owned(),
        installed: vec!["mangohud".to_owned()],
    };
    let outcome = perform(&undo, &runner);

    match outcome {
        UndoOutcome::Left { reason } => assert!(reason.contains("mangohud")),
        other @ (UndoOutcome::Reverted { .. }
        | UndoOutcome::AlreadyGone
        | UndoOutcome::Refused { .. }
        | UndoOutcome::Failed { .. }) => panic!("expected packages to be left, got {other:?}"),
    }
    assert!(runner.commands().is_empty(), "a package was removed");
}

#[test]
fn a_unit_enabled_before_the_run_is_restarted_on_its_own_config() {
    // A takeover re-pointed the unit through a drop-in; the undo removes that
    // drop-in first, so restarting the unit brings back whatever scheduler the
    // user's own configuration names.
    let runner = MockRunner::new();
    let undo = Undo::RestoreUnit {
        unit: "tuned.service".to_owned(),
        prior: crate::journal::PriorUnitState::WasEnabled,
    };
    let outcome = perform(&undo, &runner);

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert!(
        runner
            .commands()
            .iter()
            .any(|command| command.contains("systemctl restart tuned.service")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn a_unit_the_run_enabled_is_disabled_again() {
    let runner = MockRunner::new();
    let undo = Undo::RestoreUnit {
        unit: "tuned.service".to_owned(),
        prior: crate::journal::PriorUnitState::WasDisabled,
    };
    let outcome = perform(&undo, &runner);

    assert!(matches!(outcome, UndoOutcome::Reverted { .. }));
    assert_eq!(
        runner.commands(),
        ["sudo systemctl disable --now tuned.service"]
    );
}
