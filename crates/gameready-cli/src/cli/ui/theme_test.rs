use super::{Prompts, GUTTER, NOT_PICKED, PICKED};

#[test]
fn a_multi_select_marks_no_row_as_the_one_the_keyboard_is_on() {
    // The box says what is on. Where the keyboard is is the row's own weight
    // and colour, so nothing sits in the gutter to be read as a second state.
    let config = Prompts::many();

    assert_eq!(config.highlighted_option_prefix.content, GUTTER);
    assert_eq!(config.unhighlighted_option_prefix.content, GUTTER);
}

#[test]
fn a_one_of_list_draws_the_answer_as_a_dial() {
    let config = Prompts::choices();

    assert_eq!(config.highlighted_option_prefix.content, PICKED);
    assert_eq!(config.unhighlighted_option_prefix.content, NOT_PICKED);
}

#[test]
fn a_taken_choice_and_an_untaken_one_are_the_same_width() {
    // Otherwise every untaken row sits a column off from the taken one, and the
    // list appears to shuffle sideways as the answer moves down it.
    assert_eq!(
        console::measure_text_width(PICKED),
        console::measure_text_width(NOT_PICKED)
    );
}

#[test]
fn the_question_itself_carries_no_prefix() {
    // The header above it already says which question this is.
    assert_eq!(Prompts::many().prompt_prefix.content, "");
    assert_eq!(Prompts::choices().prompt_prefix.content, "");
}

#[test]
fn a_checked_box_and_an_empty_one_are_the_same_width() {
    let config = Prompts::many();

    assert_eq!(
        console::measure_text_width(config.selected_checkbox.content),
        console::measure_text_width(config.unselected_checkbox.content)
    );
}

#[test]
fn the_row_the_keyboard_is_on_is_marked_by_weight_as_well_as_colour() {
    // Colour alone would leave a terminal without it with nothing to read.
    let style = Prompts::many().selected_option.expect("a highlight style");

    assert!(style.att.contains(inquire::ui::Attributes::BOLD));
    assert!(style.fg.is_some());
}
