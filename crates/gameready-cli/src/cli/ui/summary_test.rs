use std::path::Path;
use std::time::Duration;

use gameready_core::improvement::{Check, ImprovementId, Outcome, SkipReason, Verification};
use gameready_core::journal::RunId;
use gameready_core::run::{Mode, RunReport, StepReport};

use super::*;

fn report(outcome: Outcome) -> RunReport {
    with_steps(vec![step("core.sysctl.max-map-count", outcome)])
}

fn step(id: &'static str, outcome: Outcome) -> StepReport {
    StepReport {
        step: ImprovementId::from_static(id),
        name: "Raise vm.max_map_count for Proton titles".to_owned(),
        outcome,
    }
}

fn with_steps(steps: Vec<StepReport>) -> RunReport {
    RunReport {
        run: RunId::generate(),
        mode: Mode::Apply,
        steps,
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

fn rendered(report: &RunReport, journal: &str) -> String {
    console::strip_ansi_codes(&Summary::new(report, Path::new(journal)).to_string()).into_owned()
}

#[test]
fn every_step_is_listed_with_what_it_did() {
    let text = rendered(&report(applied()), "/j");

    // Named the way the plan named it, not by the sentence the report carries.
    assert!(text.contains("vm.max_map_count"), "{text}");
    assert!(text.contains("verified"), "{text}");
}

#[test]
fn a_run_that_changed_nothing_says_the_machine_was_already_right() {
    // The bug this covers: every step already correct still printed
    // "Config changed:" over a green tick, so the run read as if it had
    // touched the machine.
    let text = rendered(&report(already_set()), "/j");

    assert!(text.contains("was already set up"), "{text}");
}

#[test]
fn a_run_that_changed_nothing_offers_no_undo_and_no_journal() {
    // Nothing was appended, so an undo command and a path to the record are
    // both offers the run cannot honour.
    let text = rendered(&report(already_set()), "/state/journal.jsonl");

    assert!(!text.contains("gameready rollback"), "{text}");
    assert!(!text.contains("/state/journal.jsonl"), "{text}");
}

#[test]
fn a_run_that_applied_something_says_so_and_offers_the_undo() {
    let text = rendered(&report(applied()), "/state/journal.jsonl");

    assert!(text.contains("Your machine is set up."), "{text}");
    assert!(text.contains("gameready rollback --run"), "{text}");
    assert!(text.contains("/state/journal.jsonl"), "{text}");
    assert!(text.contains("1 applied"), "{text}");
}

#[test]
fn a_failure_leads_the_screen_rather_than_hiding_in_the_list() {
    let failed = Outcome::Failed {
        error: "wrote 60, read back 180".to_owned(),
        rolled_back: gameready_core::improvement::RollbackStatus::Succeeded,
    };
    let text = rendered(&with_steps(vec![step("core.io.scheduler", failed)]), "/j");

    assert!(text.contains("did not land"), "{text}");
    assert!(text.contains("1 failed"), "{text}");
}

#[test]
fn the_counts_name_every_kind_of_ending_the_run_had() {
    let text = rendered(
        &with_steps(vec![
            step("core.sysctl.max-map-count", applied()),
            step("core.io.scheduler", already_set()),
        ]),
        "/j",
    );

    assert!(text.contains("1 applied · 1 already set"), "{text}");
}

#[test]
fn a_dry_run_is_not_reported_as_a_system_that_was_already_correct() {
    // A dry run leaves an applicable step unapplied. Closing with "already set
    // up" would send the user away believing there was nothing to do.
    let mut dry = report(Outcome::Skipped {
        reason: SkipReason::DryRun,
    });
    dry.mode = Mode::DryRun;
    let text = rendered(&dry, "/j");

    assert!(text.contains("Dry run"), "{text}");
    assert!(text.contains("Drop --dry-run"), "{text}");
    assert!(!text.contains("already set up"), "{text}");
    assert!(!text.contains("gameready rollback"), "{text}");
}

#[test]
fn a_failed_step_explains_itself_instead_of_squeezing_it_after_a_leader() {
    let failed = Outcome::Failed {
        error: "wrote 60, read back 180".to_owned(),
        rolled_back: gameready_core::improvement::RollbackStatus::Succeeded,
    };
    let text = rendered(&with_steps(vec![step("core.io.scheduler", failed)]), "/j");

    let broke = text
        .lines()
        .find(|line| line.contains("wrote 60"))
        .expect("what broke");
    assert!(!broke.contains(".."), "leader in a failure block: {text}");
    assert!(text.contains("I undid the partial change"), "{text}");
}

#[test]
fn a_dry_run_counts_what_dropping_the_flag_would_do() {
    // Reporting these as skips is true of the machine and no use to a reader
    // deciding whether to run it for real.
    let mut dry = with_steps(vec![
        step(
            "core.sysctl.max-map-count",
            Outcome::Skipped {
                reason: SkipReason::DryRun,
            },
        ),
        step("core.io.scheduler", already_set()),
    ]);
    dry.mode = Mode::DryRun;
    let text = rendered(&dry, "/j");

    assert!(text.contains("1 would apply · 1 already set"), "{text}");
    assert!(!text.contains("skipped"), "{text}");
}

#[test]
fn a_finished_run_says_what_to_do_next() {
    let text = rendered(&report(applied()), "/j");

    assert!(text.contains("Play something"), "{text}");
}
