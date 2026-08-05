use std::path::Path;
use std::time::Duration;

use gameready_core::improvement::{Check, ImprovementId, Outcome, Verification};
use gameready_core::journal::RunId;
use gameready_core::run::{Mode, RunReport, StepReport};

use super::*;

fn report(outcome: Outcome) -> RunReport {
    RunReport {
        run: RunId::generate(),
        mode: Mode::Apply,
        steps: vec![StepReport {
            step: ImprovementId::from_static("core.sysctl.max-map-count"),
            name: "Raise vm.max_map_count".to_owned(),
            outcome,
        }],
        installed_dependencies: Vec::new(),
        took: Duration::from_millis(300),
    }
}

#[test]
fn an_applied_step_is_marked_ok_and_says_it_was_verified() {
    let outcome = Outcome::Applied {
        changes: Vec::new(),
        verification: Verification::new().check(Check::equals("v", "1", "1")),
        took: Duration::from_millis(20),
    };
    let report = report(outcome);
    let text = Summary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    assert!(text.contains("ok Raise vm.max_map_count"));
    assert!(text.contains("verified, 1 of 1 checks passed"));
}

#[test]
fn a_failed_step_says_whether_the_change_was_undone() {
    let outcome = Outcome::Failed {
        error: "sysctl exited 1".to_owned(),
        rolled_back: gameready_core::improvement::RollbackStatus::Succeeded,
    };
    let report = report(outcome);
    let text = Summary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    assert!(text.contains("!! Raise vm.max_map_count"));
    // Without this the user cannot tell a clean failure from a half-applied one.
    assert!(text.contains("the partial change was undone"));
    assert!(text.contains("1 failed"));
}

#[test]
fn every_run_names_where_the_journal_is_and_how_to_undo() {
    let outcome = Outcome::AlreadyApplied {
        evidence: "already 2147483642".to_owned(),
    };
    let report = report(outcome);
    let text = Summary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    assert!(text.contains("gameready rollback --run"));
    assert!(text.contains("/state/journal.jsonl"));
}
