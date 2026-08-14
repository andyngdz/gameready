use super::*;

#[test]
fn the_two_answers_read_differently() {
    assert_ne!(Takeover::TakeIt.to_string(), Takeover::LeaveIt.to_string());
}

#[test]
fn switching_names_the_scheduler_the_run_would_load() {
    assert!(Takeover::TakeIt.to_string().contains("scx_lavd"));
}

#[test]
fn leaving_is_the_way_to_keep_the_owner() {
    assert!(Takeover::LeaveIt.to_string().contains("Leave"));
}
