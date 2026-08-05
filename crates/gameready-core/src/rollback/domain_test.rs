use super::*;

#[test]
fn a_left_or_refused_undo_is_not_a_failure() {
    // Leaving a package installed is the designed behaviour, not an error, so
    // it must not push the rollback's exit code non-zero.
    assert!(
        !UndoOutcome::Left {
            reason: "kept".to_owned()
        }
        .is_failure()
    );
    assert!(
        !UndoOutcome::Refused {
            reason: "edited".to_owned()
        }
        .is_failure()
    );
    assert!(!UndoOutcome::AlreadyGone.is_failure());
    assert!(
        UndoOutcome::Failed {
            error: "boom".to_owned()
        }
        .is_failure()
    );
}

#[test]
fn an_empty_plan_has_nothing_to_do() {
    let plan = RollbackPlan {
        run: crate::journal::RunId::generate(),
        undos: Vec::new(),
    };
    assert!(plan.is_empty());
}
