use std::path::Path;

use gameready_core::improvement::ImprovementId;
use gameready_core::steps::{core_steps, game_steps};

use super::*;

const NO_USER_GAMES: &str = "/nonexistent/gameready-test/games";

#[test]
fn no_step_selects_every_core_step() {
    let all = select_steps(None).expect("selects");
    assert_eq!(all.len(), core_steps().len());
}

#[test]
fn a_named_step_selects_only_that_one() {
    let selected = select_steps(Some("core.io.scheduler")).expect("selects");
    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].id(),
        ImprovementId::from_static("core.io.scheduler")
    );
}

#[test]
fn malformed_and_unknown_ids_are_errors() {
    assert!(select_steps(Some("core.does.not.exist")).is_err());
    assert!(select_steps(Some("Not A Valid Id")).is_err());
}

#[test]
fn the_selftest_sweep_includes_per_game_steps() {
    let all = select_steps_including_games(None, Path::new(NO_USER_GAMES)).expect("selects");

    assert_eq!(all.len(), core_steps().len() + game_steps().len());
    let ids: Vec<String> = all.iter().map(|step| step.id().to_string()).collect();
    assert!(
        ids.contains(&"game.steam.launch-options".to_owned()),
        "{ids:?}"
    );
    assert!(ids.contains(&"game.steam.proton".to_owned()), "{ids:?}");
}

#[test]
fn the_sweep_skips_per_game_steps_without_steam() {
    let all = select_steps_including_games(None, Path::new(NO_USER_GAMES));
    assert!(all.is_ok(), "the sweep failed instead of skipping");
}

#[test]
fn a_named_per_game_step_is_selected_when_steam_is_available() {
    let Ok(selected) =
        select_steps_including_games(Some("game.steam.proton"), Path::new(NO_USER_GAMES))
    else {
        return;
    };

    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].id(),
        ImprovementId::from_static("game.steam.proton")
    );
}
