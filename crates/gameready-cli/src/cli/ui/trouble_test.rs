use gameready_core::improvement::{Outcome, RollbackStatus, SkipReason};

use super::*;

/// Wide enough that the wrapping is the same on every machine.
const WIDTH: usize = 90;

fn rendered(mark: Mark, name: &str, outcome: &Outcome) -> String {
    let trouble = outcome.trouble().expect("an outcome that went wrong");
    let run = RunId::generate();
    let mut out = String::new();
    let mut section = Section::with_width(&mut out, WIDTH);
    WentWrong::new(mark, name, &trouble, &run)
        .write(&mut section)
        .expect("wrote into a string");
    console::strip_ansi_codes(&out).into_owned()
}

#[test]
fn a_failure_states_what_broke_and_what_state_the_machine_is_in() {
    let text = rendered(
        Mark::Failed,
        "CPU scheduler scx_lavd",
        &Outcome::Failed {
            error: "scxctl exited 1: no BPF-capable kernel headers".to_owned(),
            rolled_back: RollbackStatus::Succeeded,
        },
    );

    let lines: Vec<&str> = text.lines().collect();
    assert!(lines[0].contains("CPU scheduler scx_lavd"), "{text}");
    assert!(lines[1].contains("no BPF-capable kernel headers"), "{text}");
    assert!(lines[2].contains("exactly as it was"), "{text}");
    // Nothing to run, so nothing is offered.
    assert!(!text.contains(PROMPT), "{text}");
}

#[test]
fn an_undo_that_failed_puts_the_rollback_on_its_own_line_to_copy() {
    let text = rendered(
        Mark::Failed,
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
        .find(|line| line.contains(PROMPT))
        .expect("a command to copy");
    assert!(command.trim().starts_with(PROMPT), "{text}");
    assert!(command.contains("gameready rollback --run"), "{text}");
}

#[test]
fn a_conflict_hands_back_the_command_that_gives_the_setting_up() {
    let text = rendered(
        Mark::Skipped,
        "CPU governor",
        &Outcome::Skipped {
            reason: SkipReason::Conflict {
                with: "tuned.service".to_owned(),
                detail: "tuned.service sets the governor on its own schedule".to_owned(),
                yours: Some("systemctl disable --now tuned.service".to_owned()),
            },
        },
    );

    assert!(text.contains("behind your back"), "{text}");
    assert!(
        text.contains("systemctl disable --now tuned.service"),
        "{text}"
    );
}

#[test]
fn a_long_sentence_wraps_under_the_name_rather_than_past_the_edge() {
    let text = rendered(
        Mark::Skipped,
        "Proton-GE",
        &Outcome::Skipped {
            reason: SkipReason::CouldNotTell {
                detail: "github.com timed out after 10 seconds, and I will not pick a release \
                         from a list I could not read"
                    .to_owned(),
            },
        },
    );

    for line in text.lines() {
        assert!(
            line.chars().count() <= WIDTH,
            "over {WIDTH} columns: {line}"
        );
    }
    // Every wrapped line stays clear of the mark's gutter.
    for line in text.lines().skip(1) {
        assert!(line.starts_with("     "), "{text}");
    }
}
