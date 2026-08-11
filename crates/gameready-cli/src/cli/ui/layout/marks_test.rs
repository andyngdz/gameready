use gameready_core::improvement::OutcomeKind;

use super::*;

const EVERY_MARK: [Mark; 7] = [
    Mark::Applied,
    Mark::AlreadySet,
    Mark::Failed,
    Mark::Skipped,
    Mark::Warning,
    Mark::Recheck,
    Mark::Chosen,
];

#[test]
fn already_set_does_not_borrow_the_mark_that_means_changed() {
    assert_ne!(Mark::Applied.glyph(), Mark::AlreadySet.glyph());
}

#[test]
fn already_set_is_not_confusable_with_skipped_either() {
    assert_ne!(Mark::AlreadySet.glyph(), Mark::Skipped.glyph());
}

#[test]
fn failed_does_not_borrow_the_mark_that_means_applied() {
    assert_ne!(Mark::Failed.glyph(), Mark::Applied.glyph());
}

#[test]
fn an_update_available_install_reads_as_already_set() {
    assert_eq!(
        Mark::for_status(ProbeStatus::UpdateAvailable),
        Mark::for_status(ProbeStatus::Set)
    );
}

#[test]
fn skipped_and_not_applicable_read_the_same_because_they_mean_the_same() {
    assert_eq!(
        Mark::of(OutcomeKind::Skipped),
        Mark::of(OutcomeKind::NotApplicable)
    );
}

#[test]
fn every_outcome_maps_to_the_mark_its_name_says() {
    assert_eq!(Mark::of(OutcomeKind::Applied), Mark::Applied);
    assert_eq!(Mark::of(OutcomeKind::AlreadySet), Mark::AlreadySet);
    assert_eq!(Mark::of(OutcomeKind::Failed), Mark::Failed);
}

#[test]
fn every_mark_is_exactly_one_column_wide() {
    // Every marked line prefixes the glyph with a fixed indent and a trailing
    // space, so a two-column glyph would push one row's text past the rest.
    for mark in EVERY_MARK {
        let plain = console::strip_ansi_codes(&mark.glyph()).into_owned();
        assert_eq!(plain.chars().count(), 1, "{mark:?} is not one column");
    }
}

#[test]
fn no_two_marks_share_a_glyph() {
    // Colour is confirmation, never the only signal, so two marks that differ
    // only by colour are indistinguishable under NO_COLOR.
    let mut plain: Vec<String> = EVERY_MARK
        .iter()
        .map(|mark| console::strip_ansi_codes(&mark.glyph()).into_owned())
        .collect();
    let total = plain.len();
    plain.sort();
    plain.dedup();

    assert_eq!(plain.len(), total, "two marks look the same: {plain:?}");
}
