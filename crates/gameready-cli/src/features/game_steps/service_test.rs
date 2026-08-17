use std::path::Path;

use gameready_core::steps::{MaxMapCount, SteamLaunchOptions, SteamProton};

use super::*;

#[test]
fn both_per_game_ids_are_recognised() {
    assert!(is_game_step(&SteamLaunchOptions::id_const()));
    assert!(is_game_step(&SteamProton::id_const()));
}

#[test]
fn a_core_id_is_not_a_per_game_one() {
    assert!(!is_game_step(&MaxMapCount::id_const()));
}

#[test]
fn a_machine_without_steam_names_what_is_missing() {
    let nowhere = Path::new("/nonexistent/gameready-test/games");
    let Err(error) = build_game_step(&SteamLaunchOptions::id_const(), nowhere) else {
        return;
    };

    let text = error.to_string();
    assert!(
        text.contains("Steam") || text.contains("installed games"),
        "the error does not say what was missing: {text}"
    );
}
