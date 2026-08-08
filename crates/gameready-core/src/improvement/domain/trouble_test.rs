use std::time::Duration;

use crate::improvement::domain::identity::ImprovementId;
use crate::improvement::domain::verify::Verification;

use super::*;

fn failed(rolled_back: RollbackStatus) -> Outcome {
    Outcome::Failed {
        error: "write failed: read-only sysfs".to_owned(),
        rolled_back,
    }
}

fn skipped(reason: SkipReason) -> Outcome {
    Outcome::Skipped { reason }
}

#[test]
fn a_failure_whose_undo_worked_says_the_machine_is_as_it_was() {
    let trouble = failed(RollbackStatus::Succeeded)
        .trouble()
        .expect("trouble");

    assert_eq!(trouble.broke, "write failed: read-only sysfs");
    assert!(trouble.now.contains("exactly as it was"), "{trouble:?}");
    // The good news is the missing line: nothing to run.
    assert_eq!(trouble.fix, None);
}

#[test]
fn a_failure_whose_undo_also_failed_offers_the_rollback() {
    let trouble = failed(RollbackStatus::Failed {
        detail: "sysfs still read-only".to_owned(),
    })
    .trouble()
    .expect("trouble");

    assert!(trouble.now.contains("may still be in place"), "{trouble:?}");
    assert!(
        matches!(trouble.fix, Some(Remedy::Rollback { .. })),
        "{trouble:?}"
    );
}

#[test]
fn a_failure_that_wrote_nothing_says_nothing_was_left_behind() {
    let trouble = failed(RollbackStatus::NotAttempted)
        .trouble()
        .expect("trouble");

    assert_eq!(trouble.now, NOTHING_LEFT);
    assert_eq!(trouble.fix, None);
}

#[test]
fn a_conflict_names_the_owner_and_hands_the_decision_back() {
    let trouble = skipped(SkipReason::Conflict {
        with: "tuned.service".to_owned(),
        detail: "tuned.service sets the governor on its own schedule".to_owned(),
        yours: Some("systemctl disable --now tuned.service".to_owned()),
    })
    .trouble()
    .expect("trouble");

    assert!(trouble.broke.contains("tuned.service"), "{trouble:?}");
    assert!(trouble.now.contains("left it alone"), "{trouble:?}");
    match trouble.fix {
        Some(Remedy::Yours { command, .. }) => {
            assert_eq!(command, "systemctl disable --now tuned.service");
        }
        other => panic!("expected a command to hand back: {other:?}"),
    }
}

#[test]
fn a_conflict_with_no_single_command_offers_none() {
    // Unloading a scheduler somebody else started is not one command, and
    // inventing one would be worse than saying nothing.
    let trouble = skipped(SkipReason::Conflict {
        with: "bpfland".to_owned(),
        detail: "bpfland is already scheduling this machine".to_owned(),
        yours: None,
    })
    .trouble()
    .expect("trouble");

    assert_eq!(trouble.fix, None);
}

#[test]
fn a_probe_that_could_not_read_says_it_guessed_at_nothing() {
    let trouble = skipped(SkipReason::CouldNotTell {
        detail: "github.com timed out".to_owned(),
    })
    .trouble()
    .expect("trouble");

    assert!(
        trouble.broke.contains("github.com timed out"),
        "{trouble:?}"
    );
    assert!(
        trouble.now.contains("skip rather than guess"),
        "{trouble:?}"
    );
}

#[test]
fn a_step_left_out_by_a_broken_one_names_the_step_that_broke() {
    let trouble = skipped(SkipReason::DependencyFailed {
        on: ImprovementId::from_static("core.repo.scx-ppa"),
    })
    .trouble()
    .expect("trouble");

    assert!(trouble.broke.contains("core.repo.scx-ppa"), "{trouble:?}");
}

#[test]
fn a_step_the_user_declined_is_a_choice_rather_than_a_trouble() {
    assert_eq!(skipped(SkipReason::UserDeclined).trouble(), None);
    assert_eq!(skipped(SkipReason::DryRun).trouble(), None);
}

#[test]
fn an_ending_that_went_right_has_nothing_to_explain() {
    let applied = Outcome::Applied {
        changes: Vec::new(),
        verification: Verification::new(),
        took: Duration::from_millis(1),
    };

    assert_eq!(applied.trouble(), None);
    assert_eq!(
        Outcome::NotApplicable {
            reason: "this hardware offers no performance governor".to_owned(),
        }
        .trouble(),
        None
    );
}
