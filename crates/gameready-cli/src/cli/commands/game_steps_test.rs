use gameready_core::steps::{MaxMapCount, SteamLaunchOptions, SteamProton};

use super::*;

#[test]
fn both_per_game_ids_are_recognised() {
    assert!(is_game_step(&SteamLaunchOptions::id_const()));
    assert!(is_game_step(&SteamProton::id_const()));
}

#[test]
fn a_core_id_is_not_a_per_game_one() {
    // The two lists have to stay apart: a core id routed through the per-game
    // builder would go looking for Steam to test a kernel setting.
    assert!(!is_game_step(&MaxMapCount::id_const()));
}

#[test]
fn a_machine_without_steam_says_so_rather_than_reporting_a_skip() {
    // A step built with no config path probes as not-applicable, which reads in
    // the summary as "nothing to test here" rather than "there was nothing to
    // test it against". The difference matters: the first is a pass.
    let nowhere = Path::new("/nonexistent/gameready-test/games");
    let Err(error) = build_game_step(&SteamLaunchOptions::id_const(), nowhere) else {
        // A developer machine with Steam installed reaches the other branch,
        // and that is not this test's business.
        return;
    };

    let text = error.to_string();
    assert!(
        text.contains("Steam") || text.contains("profile"),
        "the error does not say what was missing: {text}"
    );
}
