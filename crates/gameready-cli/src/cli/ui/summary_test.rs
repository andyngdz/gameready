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
fn every_step_is_listed_with_its_detail() {
    let outcome = Outcome::Applied {
        changes: Vec::new(),
        verification: Verification::new().check(Check::equals("v", "1", "1")),
        took: Duration::from_millis(20),
    };
    let text = Summary::new(&report(outcome), Path::new("/j")).to_string();

    assert!(text.contains("Raise vm.max_map_count"), "{text}");
    assert!(text.contains("verified"), "{text}");
}

#[test]
fn the_footer_shows_counts_and_journal() {
    let outcome = Outcome::AlreadyApplied {
        evidence: "already 2147483642".to_owned(),
    };
    let text = Summary::new(&report(outcome), Path::new("/state/journal.jsonl")).to_string();

    assert!(text.contains("already set up"), "{text}");
    assert!(text.contains("/state/journal.jsonl"), "{text}");
}
