use std::time::Duration;

use gameready_core::improvement::{Check, Outcome, RollbackStatus, SkipReason, Verification};

use super::*;

/// Wide enough that the wrapping is the same on every machine. Matches the
/// widest layout the real width clamps to, so a one-row result stays one row.
const WIDTH: usize = 100;

/// A name column any of these test names fits inside.
const COLUMN: usize = 24;

fn rendered(name: &str, outcome: &Outcome) -> String {
    let run = RunId::generate();
    let mut out = String::new();
    let mut section = Section::with_width(&mut out, WIDTH);
    StepRow {
        mark: Mark::of(outcome.kind()),
        name,
        outcome,
        column: COLUMN,
        run: &run,
    }
    .write(&mut section)
    .expect("wrote into a string");
    console::strip_ansi_codes(&out).into_owned()
}

fn already_set() -> Outcome {
    Outcome::AlreadyApplied {
        evidence: "already 2147483642".to_owned(),
    }
}

#[test]
fn a_step_that_landed_is_one_row_and_nothing_under_it() {
    let text = rendered(
        "vm.max_map_count",
        &Outcome::Applied {
            changes: Vec::new(),
            verification: Verification::new().check(Check::equals("v", "1", "1")),
            took: Duration::from_millis(20),
        },
    );

    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.contains("verified"), "{text}");
}

#[test]
fn an_already_set_step_is_one_row() {
    let text = rendered("vm.max_map_count", &already_set());

    assert_eq!(text.lines().count(), 1, "{text}");
    assert!(text.contains("already 2147483642"), "{text}");
}

#[test]
fn a_conflict_is_a_row_with_its_command_under_it() {
    // The bug this covers: a conflict used to break into a four-line block
    // while every neighbour stayed a single aligned row.
    let text = rendered(
        "CPU governor",
        &Outcome::Skipped {
            reason: SkipReason::Conflict {
                with: "tuned.service".to_owned(),
                detail: "tuned.service sets the governor on its own schedule".to_owned(),
                yours: Some("systemctl disable --now tuned.service".to_owned()),
            },
        },
    );

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].contains("I left it to tuned.service"), "{text}");
    // The command sits under the name, not behind a prompt glyph.
    assert_eq!(
        lines[1], "    systemctl disable --now tuned.service",
        "{text}"
    );
    assert!(!text.contains('❯'), "{text}");
    // The framing sentences of a failure block have no place on a row.
    assert!(!text.contains("behind your back"), "{text}");
    assert!(!text.contains("left it alone"), "{text}");
}

#[test]
fn a_conflict_with_no_command_is_just_the_row() {
    let text = rendered(
        "Competing daemons",
        &Outcome::Skipped {
            reason: SkipReason::Conflict {
                with: "power-profiles-daemon.service".to_owned(),
                detail: "power-profiles-daemon.service is running".to_owned(),
                yours: None,
            },
        },
    );

    assert_eq!(text.lines().count(), 1, "{text}");
}

#[test]
fn a_failure_still_states_what_broke_and_the_state_it_left() {
    let text = rendered(
        "CPU scheduler scx_lavd",
        &Outcome::Failed {
            error: "scxctl exited 1: no BPF-capable kernel headers".to_owned(),
            rolled_back: RollbackStatus::Succeeded,
        },
    );

    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 3, "{text}");
    assert!(lines[0].contains("CPU scheduler scx_lavd"), "{text}");
    assert!(lines[1].contains("no BPF-capable kernel headers"), "{text}");
    assert!(lines[2].contains("exactly as it was"), "{text}");
}

#[test]
fn an_undo_that_failed_offers_the_rollback_to_retry() {
    let text = rendered(
        "I/O scheduler · sdb",
        &Outcome::Failed {
            error: "write failed: read-only sysfs".to_owned(),
            rolled_back: RollbackStatus::Failed {
                detail: "sysfs still read-only".to_owned(),
            },
        },
    );

    let command = text
        .lines()
        .find(|line| line.contains("gameready rollback --run"))
        .expect("a command to copy");
    // Aligned under the name, no prompt glyph in front of it.
    assert!(
        command.starts_with("    gameready rollback --run"),
        "{text}"
    );
    assert!(!text.contains('❯'), "{text}");
}
