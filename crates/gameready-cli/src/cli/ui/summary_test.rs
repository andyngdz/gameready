use std::path::Path;
use std::time::Duration;

use gameready_core::improvement::{Check, ImprovementId, Outcome, SkipReason, Verification};
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

fn applied() -> Outcome {
    Outcome::Applied {
        changes: Vec::new(),
        verification: Verification::new().check(Check::equals("v", "1", "1")),
        took: Duration::from_millis(20),
    }
}

fn already_set() -> Outcome {
    Outcome::AlreadyApplied {
        evidence: "already 2147483642".to_owned(),
    }
}

#[test]
fn every_step_is_listed_with_its_detail() {
    let text = Summary::new(&report(applied()), Path::new("/j")).to_string();

    assert!(text.contains("Raise vm.max_map_count"), "{text}");
    assert!(text.contains("verified"), "{text}");
}

#[test]
fn a_run_that_changed_nothing_does_not_head_the_list_with_config_changed() {
    // The bug this covers: every step already correct still printed
    // "Config changed:" over a green tick, so the run read as if it had
    // touched the machine.
    let text = Summary::new(&report(already_set()), Path::new("/j")).to_string();

    assert!(text.trim_start().starts_with("Nothing changed:"), "{text}");
    assert!(text.contains("already set up"), "{text}");
}

#[test]
fn a_run_that_changed_nothing_offers_no_undo_and_no_journal() {
    // Nothing was appended, so an undo command and a path to the record are
    // both offers the run cannot honour.
    let text = Summary::new(&report(already_set()), Path::new("/state/journal.jsonl")).to_string();

    assert!(!text.contains("Rollback saved"), "{text}");
    assert!(!text.contains("/state/journal.jsonl"), "{text}");
}

#[test]
fn a_run_that_applied_something_says_so_and_offers_the_undo() {
    let text = Summary::new(&report(applied()), Path::new("/state/journal.jsonl")).to_string();

    assert!(text.trim_start().starts_with("Config changed:"), "{text}");
    assert!(text.contains("gameready rollback --run"), "{text}");
    assert!(text.contains("/state/journal.jsonl"), "{text}");
    assert!(text.contains("1 applied"), "{text}");
}

#[test]
fn a_dry_run_is_not_reported_as_a_system_that_was_already_correct() {
    // A dry run leaves an applicable step unapplied. Closing with "already set
    // up" would send the user away believing there was nothing to do.
    let mut dry = report(Outcome::Skipped {
        reason: SkipReason::DryRun,
    });
    dry.mode = Mode::DryRun;
    let text = Summary::new(&dry, Path::new("/j")).to_string();

    assert!(text.contains("Dry run"), "{text}");
    assert!(!text.contains("already set up"), "{text}");
    assert!(!text.contains("Rollback saved"), "{text}");
}
