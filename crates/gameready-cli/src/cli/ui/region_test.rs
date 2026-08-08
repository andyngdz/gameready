use super::*;

#[test]
fn both_templates_are_ones_indicatif_accepts() {
    // The style is applied only when it parses, so a typo in a placeholder
    // would silently leave the default style rather than fail.
    assert!(ProgressStyle::with_template(SPINNER).is_ok());
    assert!(ProgressStyle::with_template(COUNTING).is_ok());
}

#[test]
fn a_spinner_replaces_whatever_was_live_rather_than_stacking_under_it() {
    let mut region = LiveRegion::default();

    region.spin("Fetching latest release info".to_owned());
    region.spin("Downloading GE-Proton11-3.tar.gz".to_owned());

    assert!(region.is_live());
}

#[test]
fn every_report_of_the_same_transfer_moves_one_bar() {
    // Three thousand reports arrive for a 178 MB download. Three thousand bars
    // would be three thousand lines.
    let mut region = LiveRegion::default();

    region.count("Downloading", 0, 186_703_872);
    let first = region.counting();
    region.count("Downloading", 65_536, 186_703_872);
    region.count("Downloading", 131_072, 186_703_872);

    assert_eq!(region.counting(), first);
    assert_eq!(region.counting(), Some(186_703_872));
}

#[test]
fn a_second_transfer_gets_its_own_bar() {
    let mut region = LiveRegion::default();

    region.count("Downloading", 100, 1_000);
    region.count("Downloading something else", 5, 2_000);

    assert_eq!(region.counting(), Some(2_000));
}

#[test]
fn settling_hands_a_single_line_to_the_bar_and_takes_a_wrapped_one_back() {
    let mut region = LiveRegion::default();

    region.spin("Downloading".to_owned());
    assert!(region.settle("  ✓ Proton-GE ... installed"));

    region.spin("Downloading".to_owned());
    assert!(!region.settle("  ✘ Proton-GE\n     github.com timed out"));
}

#[test]
fn settling_a_download_takes_the_bar_away_rather_than_leaving_it_under_the_row() {
    let mut region = LiveRegion::default();

    region.count("Proton-GE", 186_703_872, 186_703_872);

    assert!(!region.settle("  ✓ Proton-GE ... installed"));
    assert_eq!(region.counting(), None);
}

#[test]
fn settling_with_nothing_live_leaves_the_line_to_the_caller() {
    let mut region = LiveRegion::default();

    assert!(!region.settle("  • Swappiness ... already 180"));
}

#[test]
fn clearing_forgets_the_total_so_the_next_transfer_starts_over() {
    let mut region = LiveRegion::default();

    region.count("Downloading", 100, 1_000);
    region.clear();

    assert_eq!(region.counting(), None);
    assert!(!region.is_live());
}
