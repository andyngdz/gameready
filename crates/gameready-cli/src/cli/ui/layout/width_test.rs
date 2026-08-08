use super::*;

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
