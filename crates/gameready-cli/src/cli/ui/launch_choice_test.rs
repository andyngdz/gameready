use super::LaunchChoice;

#[test]
fn each_choice_says_what_it_will_do_to_steam() {
    // The user is agreeing to have their game client closed, so the option that
    // does it has to say so before they pick it.
    assert!(
        LaunchChoice::CloseSteamAndWrite
            .to_string()
            .contains("Close Steam")
    );
}

#[test]
fn the_hands_off_choice_promises_nothing_will_be_touched() {
    let shown = LaunchChoice::ShowForCopying.to_string();
    assert!(shown.contains("paste"), "{shown}");
    assert!(!shown.contains("Close Steam"), "{shown}");
}

#[test]
fn the_two_choices_read_differently() {
    assert_ne!(
        LaunchChoice::CloseSteamAndWrite.to_string(),
        LaunchChoice::ShowForCopying.to_string()
    );
}
