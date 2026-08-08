use gameready_core::improvement::OutcomeKind;

use super::*;

/// A width narrow enough that a long row cannot fit its leader.
const CRAMPED: usize = 60;

/// A width wide enough for a leader and some room either side.
const ROOMY: usize = 80;

#[test]
fn already_set_does_not_borrow_the_mark_that_means_changed() {
    let applied = outcome_mark(OutcomeKind::Applied);
    let already = outcome_mark(OutcomeKind::AlreadySet);
    assert_ne!(applied, already);
}

#[test]
fn already_set_is_not_confusable_with_skipped_either() {
    assert_ne!(
        outcome_mark(OutcomeKind::AlreadySet),
        outcome_mark(OutcomeKind::Skipped)
    );
}

#[test]
fn skipped_and_not_applicable_use_the_same_mark() {
    let skipped = outcome_mark(OutcomeKind::Skipped);
    let na = outcome_mark(OutcomeKind::NotApplicable);
    assert_eq!(skipped, na);
}

#[test]
fn failed_mark_differs_from_applied() {
    assert_ne!(
        outcome_mark(OutcomeKind::Failed),
        outcome_mark(OutcomeKind::Applied)
    );
}

#[test]
fn every_mark_is_one_column_wide() {
    let marks = [
        outcome_mark(OutcomeKind::Applied),
        outcome_mark(OutcomeKind::AlreadySet),
        outcome_mark(OutcomeKind::Failed),
        outcome_mark(OutcomeKind::Skipped),
        warning_mark(),
    ];
    for mark in marks {
        assert_eq!(console::strip_ansi_codes(&mark).chars().count(), 1);
    }
}

#[test]
fn a_terminal_wider_than_prose_reads_well_is_clamped_back() {
    assert_eq!(usable(200), WIDEST);
}

#[test]
fn a_terminal_too_narrow_for_the_label_column_is_widened_to_the_floor() {
    assert_eq!(usable(20), NARROWEST);
}

#[test]
fn a_normal_terminal_width_is_taken_as_it_is() {
    assert_eq!(usable(88), 88);
}

#[test]
fn section_title_adds_a_blank_line() {
    let mut buf = String::new();
    Section::new(&mut buf).title("Test:").unwrap();
    assert_eq!(buf, "Test:\n\n");
}

#[test]
fn marked_line_is_indented_with_mark() {
    let mut buf = String::new();
    Section::new(&mut buf).marked("*", "hello").unwrap();
    assert_eq!(buf, "  * hello\n");
}

#[test]
fn indented_line_has_no_mark() {
    let mut buf = String::new();
    Section::new(&mut buf).indented("text").unwrap();
    assert_eq!(buf, "  text\n");
}

#[test]
fn the_separator_is_as_wide_as_the_layout() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY).end().unwrap();
    let rule = console::strip_ansi_codes(buf.trim_end()).into_owned();
    assert_eq!(rule.len(), ROOMY);
}

#[test]
fn a_row_fills_the_width_so_the_evidence_lands_on_the_right_edge() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY)
        .row("+", "Raise vm.max_map_count", Some("65530 to 2147483642"))
        .unwrap();

    let line = console::strip_ansi_codes(buf.trim_end()).into_owned();
    assert_eq!(line.chars().count(), ROOMY);
    assert!(line.starts_with("  + Raise vm.max_map_count ."));
    assert!(line.ends_with(". 65530 to 2147483642"));
}

#[test]
fn a_row_with_no_evidence_is_just_a_marked_line() {
    let mut buf = String::new();
    Section::with_width(&mut buf, ROOMY)
        .row("+", "Competing daemons", None)
        .unwrap();
    assert_eq!(
        console::strip_ansi_codes(&buf).trim_end(),
        "  + Competing daemons"
    );
}

#[test]
fn evidence_with_no_room_for_a_leader_drops_to_its_own_line() {
    let mut buf = String::new();
    Section::with_width(&mut buf, CRAMPED)
        .row(
            "+",
            "I/O scheduler for the boot disk nvme0n1",
            Some("mq-deadline to none"),
        )
        .unwrap();

    let plain = console::strip_ansi_codes(&buf).into_owned();
    let lines: Vec<&str> = plain.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(!lines[0].contains('.'));
    assert_eq!(lines[1], "     mq-deadline to none");
}

#[test]
fn a_labelled_paragraph_wraps_inside_the_layout_width() {
    let mut buf = String::new();
    let prose = "Steam rewrites its own config when it quits, so it has to close \
                 before anything writes launch options into it.";
    Section::with_width(&mut buf, CRAMPED)
        .labelled("Why", prose)
        .unwrap();

    let plain = console::strip_ansi_codes(&buf).into_owned();
    assert!(plain.lines().count() > 1);
    for line in plain.lines() {
        assert!(line.chars().count() <= CRAMPED, "too long: {line}");
    }
}

#[test]
fn only_the_first_line_of_a_labelled_paragraph_carries_the_label() {
    let mut buf = String::new();
    Section::with_width(&mut buf, CRAMPED)
        .labelled("Why", &"word ".repeat(30))
        .unwrap();

    let plain = console::strip_ansi_codes(&buf).into_owned();
    let labelled: Vec<&str> = plain.lines().filter(|line| line.contains("Why")).collect();
    assert_eq!(labelled.len(), 1);
}
