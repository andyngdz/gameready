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
fn a_pass_leads_with_its_marker() {
    let results = [result(SelftestResult::Passed {
        reverted: RevertCheck::Confirmed,
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    assert!(
        rendered.contains("ok  core.sysctl.max-map-count"),
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

    assert!(rendered.contains("!!"), "{rendered}");
    assert!(rendered.contains("rollback failed"), "{rendered}");
    assert!(rendered.contains("the undo command exited 1"), "{rendered}");
}

#[test]
fn a_skip_says_why() {
    let results = [result(SelftestResult::Skipped {
        reason: "already set (vm.max_map_count is already 2147483642)".to_owned(),
    })];
    let rendered = SelftestSummary::new(&results).to_string();

    assert!(rendered.contains("--"), "{rendered}");
    assert!(rendered.contains("already set"), "{rendered}");
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
fn every_result_gets_one_line() {
    let results = [
        result(SelftestResult::Passed {
            reverted: RevertCheck::Confirmed,
        }),
        result(SelftestResult::ProbeFailed {
            error: "permission denied".to_owned(),
        }),
    ];
    let rendered = SelftestSummary::new(&results).to_string();

    // One header line, one blank line before it, and one line per result.
    assert_eq!(rendered.lines().filter(|line| !line.is_empty()).count(), 3);
}
