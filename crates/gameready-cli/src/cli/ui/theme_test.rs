use super::{questions, CURSOR, NO_CURSOR};

#[test]
fn the_cursor_and_its_absence_take_the_same_width() {
    // Otherwise every row without the cursor sits a column left of the one with
    // it, and the list appears to shuffle sideways as the cursor moves.
    assert_eq!(
        console::measure_text_width(CURSOR),
        console::measure_text_width(NO_CURSOR)
    );
}

#[test]
fn the_question_itself_carries_no_prefix() {
    // The header above it already says which question this is.
    let config = questions();

    assert_eq!(config.prompt_prefix.content, "");
}

#[test]
fn a_checked_box_and_an_empty_one_are_the_same_width() {
    let config = questions();

    assert_eq!(
        console::measure_text_width(config.selected_checkbox.content),
        console::measure_text_width(config.unselected_checkbox.content)
    );
}
