use gameready_core::improvement::OutcomeKind;

use super::*;

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
fn end_writes_a_separator() {
    let mut buf = String::new();
    Section::new(&mut buf).end().unwrap();
    assert!(!buf.is_empty());
    assert!(buf.ends_with('\n'));
}
