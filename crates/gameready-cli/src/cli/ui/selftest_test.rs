use gameready_core::improvement::ImprovementId;
use gameready_core::run::{Phase, RevertCheck, SelftestResult, StepSelftest};

use super::SelftestSummary;

fn result(result: SelftestResult) -> StepSelftest {
    StepSelftest {
        step: ImprovementId::from_static("core.sysctl.max-map-count"),
        result,
    }
}

#[test]
fn a_pass_reads_as_the_whole_cycle() {
    let results = [result(SelftestResult::Passed {
        reverted: RevertCheck::Confirmed,
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    // The step is named by its short name, not the id it was recorded under.
    assert!(rendered.contains("vm.max_map_count"), "{rendered}");
    assert!(
        rendered.contains("applied, verified, reverted"),
        "{rendered}"
    );
}

#[test]
fn a_failure_names_the_phase_and_the_detail() {
    let results = [result(SelftestResult::Failed {
        phase: Phase::Rollback,
        detail: "the undo command exited 1".to_owned(),
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    assert!(rendered.contains("rollback failed"), "{rendered}");
    assert!(rendered.contains("the undo command exited 1"), "{rendered}");
    assert!(rendered.contains("1 of 1 failed"), "{rendered}");
}

#[test]
fn the_verdict_carries_the_reassurance_rather_than_standing_above_it() {
    let results = [result(SelftestResult::Failed {
        phase: Phase::Rollback,
        detail: "the undo command exited 1".to_owned(),
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    let verdict = rendered
        .lines()
        .find(|line| line.contains("1 of 1 failed"))
        .expect("the verdict");
    assert!(verdict.contains("Your machine is as it was"), "{rendered}");
}

#[test]
fn a_skip_says_why() {
    let results = [result(SelftestResult::Skipped {
        reason: "already set (vm.max_map_count is already 2147483642)".to_owned(),
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    assert!(rendered.contains("skipped, already set"), "{rendered}");
}

#[test]
fn a_step_with_nothing_to_revert_says_so_rather_than_claiming_a_readback() {
    let results = [result(SelftestResult::Passed {
        reverted: RevertCheck::NotApplicable,
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    assert!(rendered.contains("nothing to revert"), "{rendered}");
}

#[test]
fn every_result_gets_its_own_line() {
    let results = [
        result(SelftestResult::Passed {
            reverted: RevertCheck::Confirmed,
        }),
        result(SelftestResult::ProbeFailed {
            error: "permission denied".to_owned(),
        }),
    ];
    let rendered = SelftestSummary::new(&results).to_string();

    // Each result reads on its own line, so neither is folded into the other.
    assert!(
        rendered.contains("applied, verified, reverted"),
        "{rendered}"
    );
    assert!(
        rendered.contains("probe failed: permission denied"),
        "{rendered}"
    );
    assert!(rendered.contains("1 of 2 failed"), "{rendered}");
}
