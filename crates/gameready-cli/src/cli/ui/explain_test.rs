use gameready_core::improvement::{ImprovementId, PlannedAction, StepPlan};

use super::*;

fn explanation(found: &str, plan: Option<StepPlan>) -> StepExplanation {
    StepExplanation {
        id: "core.memory.swappiness".to_owned(),
        name: "Raise vm.swappiness for zram swap".to_owned(),
        rationale: "When swap lives in zram, a swapped-out page is compressed into RAM rather \
                    than written to disk, so swapping early frees more usable memory at almost \
                    no cost."
            .to_owned(),
        privilege: Privilege::Root,
        gains: Some("More usable memory under pressure, at almost no cost.".to_owned()),
        undo_note: None,
        found: found.to_owned(),
        plan,
    }
}

fn plan() -> StepPlan {
    StepPlan::new(
        ImprovementId::from_static("core.memory.swappiness"),
        "vm.swappiness 60 -> 180",
    )
    .action(PlannedAction::SetSysctl {
        key: "vm.swappiness".to_owned(),
        from: "60".to_owned(),
        to: "180".to_owned(),
    })
}

#[test]
fn a_step_that_applies_here_shows_what_it_would_do() {
    let rendered = explanation("applicable", Some(plan())).to_string();

    assert!(rendered.contains("Would do"), "{rendered}");
    assert!(
        rendered.contains("set vm.swappiness from 60 to 180"),
        "{rendered}"
    );
}

#[test]
fn a_step_that_does_not_apply_here_shows_no_empty_plan() {
    // An empty "would do" reads as gameready being about to do nothing, rather
    // than as the step not being for this machine.
    let rendered = explanation("not applicable: swap is on disk, not zram", None).to_string();

    assert!(!rendered.contains("Would do"), "{rendered}");
    assert!(rendered.contains("swap is on disk"), "{rendered}");
}

#[test]
fn the_reason_wraps_instead_of_running_off_the_screen() {
    let rendered = explanation("applicable", None).to_string();

    let width = crate::cli::ui::layout::width();
    for line in rendered.lines() {
        let plain = console::strip_ansi_codes(line);
        assert!(plain.chars().count() <= width, "too long: {line}");
    }
}

#[test]
fn a_root_step_says_it_will_ask_for_a_password() {
    // The one thing a user wants to know before running something is whether it
    // touches the system or only their own files.
    let rendered = explanation("applicable", None).to_string();

    assert!(rendered.contains("password"), "{rendered}");
}

#[test]
fn a_user_level_step_says_it_stays_inside_the_home_directory() {
    let mut user_level = explanation("applicable", None);
    user_level.privilege = Privilege::User;

    let rendered = user_level.to_string();
    assert!(rendered.contains("your own files"), "{rendered}");
    assert!(!rendered.contains("password"), "{rendered}");
}

#[test]
fn every_step_is_listed_with_the_command_that_explains_it() {
    let rendered = StepIndex::all().to_string();

    assert!(rendered.contains("core.memory.swappiness"), "{rendered}");
    assert!(rendered.contains("gameready explain"), "{rendered}");
}

#[test]
fn the_undo_line_names_the_command_rather_than_promising_reversibility() {
    // "Undoable" is a claim; the command is something the reader can run.
    let rendered = explanation("applicable", None).to_string();

    assert!(rendered.contains("gameready rollback"), "{rendered}");
}
