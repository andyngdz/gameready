use super::{LaunchChoice, SteamWork};

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
    assert!(shown.contains("myself"), "{shown}");
    assert!(!shown.contains("Close Steam"), "{shown}");
}

#[test]
fn the_two_choices_read_differently() {
    assert_ne!(
        LaunchChoice::CloseSteamAndWrite.to_string(),
        LaunchChoice::ShowForCopying.to_string()
    );
}

#[test]
fn the_question_names_the_proton_version_rather_than_hiding_it_in_settings() {
    // It is the setting a user is most likely to have chosen themselves, so it
    // should not arrive as a surprise inside a word like "settings".
    let question = SteamWork {
        launch: 3,
        proton: 2,
    }
    .question();

    assert!(question.contains("launch options for 3"), "{question}");
    assert!(question.contains("Proton version for 2"), "{question}");
}

#[test]
fn a_run_with_no_pins_does_not_mention_proton_at_all() {
    let question = SteamWork {
        launch: 3,
        proton: 0,
    }
    .question();

    assert!(question.contains("launch options for 3"), "{question}");
    assert!(!question.contains("Proton"), "{question}");
}
