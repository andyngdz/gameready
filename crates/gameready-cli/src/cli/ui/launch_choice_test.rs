use super::{LaunchChoice, SteamWork};

#[test]
fn each_choice_says_what_it_will_do_to_steam() {
    // The user is agreeing to have their game client closed, so the option that
    // does it has to say so before they pick it.
    assert!(LaunchChoice::CloseSteamAndWrite
        .to_string()
        .contains("Close Steam"));
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
fn the_question_counts_the_games_it_is_about() {
    let question = SteamWork {
        launch: 3,
        proton: 2,
    }
    .question();

    assert!(question.contains("3 games"), "{question}");
}

#[test]
fn one_game_is_not_asked_about_as_one_games() {
    let question = SteamWork {
        launch: 1,
        proton: 0,
    }
    .question();

    assert!(question.contains("1 game:"), "{question}");
}

#[test]
fn the_detail_names_the_proton_build_rather_than_hiding_it_in_settings() {
    // It is the setting a user is most likely to have chosen themselves, so it
    // should not arrive as a surprise inside a word like "settings".
    let detail = SteamWork {
        launch: 3,
        proton: 2,
    }
    .detail();

    assert!(detail.contains("Launch options for 3"), "{detail}");
    assert!(detail.contains("Proton build for 2"), "{detail}");
}

#[test]
fn a_run_with_no_pins_does_not_mention_proton_at_all() {
    let detail = SteamWork {
        launch: 3,
        proton: 0,
    }
    .detail();

    assert!(detail.contains("Launch options for 3"), "{detail}");
    assert!(!detail.contains("Proton"), "{detail}");
}

#[test]
fn the_detail_says_why_steam_has_to_close() {
    let detail = SteamWork {
        launch: 2,
        proton: 0,
    }
    .detail();

    assert!(detail.contains("has to close first"), "{detail}");
}
