use super::*;

fn result(result: SelftestResult) -> StepSelftest {
    StepSelftest {
        step: ImprovementId::from_static("core.test"),
        result,
    }
}

#[test]
fn a_skipped_step_is_not_a_failure() {
    // A machine that cannot take a step has told us something true about
    // itself, not about the step.
    let skipped = result(SelftestResult::Skipped {
        reason: "already set".to_owned(),
    });
    assert!(!skipped.is_failure());
}

#[test]
fn a_passing_step_is_not_a_failure() {
    let passed = result(SelftestResult::Passed {
        reverted: RevertCheck::Confirmed,
    });
    assert!(!passed.is_failure());
}

#[test]
fn a_failed_phase_is_a_failure() {
    let failed = result(SelftestResult::Failed {
        phase: Phase::Rollback,
        detail: "the undo command exited 1".to_owned(),
    });
    assert!(failed.is_failure());
}

#[test]
fn a_probe_that_could_not_run_is_a_failure() {
    // A step that cannot read the current state cannot restore it either.
    let failed = result(SelftestResult::ProbeFailed {
        error: "permission denied".to_owned(),
    });
    assert!(failed.is_failure());
}

#[test]
fn every_phase_and_revert_check_labels_itself() {
    for phase in [
        Phase::Apply,
        Phase::Verify,
        Phase::Rollback,
        Phase::Reverted,
    ] {
        assert!(!phase.label().is_empty());
    }
    for check in [RevertCheck::Confirmed, RevertCheck::NotApplicable] {
        assert!(!check.label().is_empty());
    }
}
