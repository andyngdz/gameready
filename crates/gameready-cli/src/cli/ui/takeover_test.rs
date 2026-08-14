use super::*;

#[test]
fn the_two_answers_read_differently() {
    assert_ne!(Takeover::TakeIt.to_string(), Takeover::LeaveIt.to_string());
}

#[test]
fn taking_over_names_the_takeover() {
    assert!(Takeover::TakeIt.to_string().contains("Take it over"));
}

#[test]
fn leaving_is_the_way_to_keep_the_owner() {
    assert!(Takeover::LeaveIt.to_string().contains("Leave"));
}
