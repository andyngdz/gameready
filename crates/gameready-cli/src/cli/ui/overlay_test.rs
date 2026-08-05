use gameready_core::steam::Overlay;

#[test]
fn the_two_answers_are_distinct_states() {
    // A bool would read as `false` at the call site and say nothing about what
    // the user was asked.
    assert_ne!(Overlay::Show, Overlay::Hide);
}

#[test]
fn hide_is_the_safe_default() {
    // Documented here so a later change to the prompt's default has to change a
    // test that says why, rather than sliding past review.
    assert_eq!(Overlay::default_answer(), Overlay::Hide);
}
