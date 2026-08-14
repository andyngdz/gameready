use super::*;

#[test]
fn no_step_selects_every_step() {
    let all = select_steps(None).expect("selects");
    assert_eq!(all.len(), core_steps().len());
}

#[test]
fn a_named_step_selects_only_that_one() {
    let selected = select_steps(Some("core.io.scheduler")).expect("selects");
    assert_eq!(selected.len(), 1);
    let only = &selected[0];
    assert_eq!(only.id(), ImprovementId::from_static("core.io.scheduler"));
}

#[test]
fn an_unknown_but_well_formed_step_id_is_an_error() {
    assert!(select_steps(Some("core.does.not.exist")).is_err());
}

#[test]
fn a_malformed_step_id_is_an_error() {
    assert!(select_steps(Some("Not A Valid Id")).is_err());
}

/// A games directory that cannot exist, so the built-in catalog is all there is.
const NO_USER_GAMES: &str = "/nonexistent/gameready-test/games";

#[test]
fn the_selftest_sweep_covers_every_step_including_the_per_game_ones() {
    // "All passed" has to mean all of them. Leaving two out of the sweep would
    // make the headline narrower than it reads.
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
fn the_sweep_still_answers_on_a_machine_with_no_steam() {
    // A container has no Steam, and the distro CI job asserts a clean exit. The
    // per-game steps have to come back inert there rather than failing the call.
    let all = select_steps_including_games(None, Path::new(NO_USER_GAMES));

    assert!(all.is_ok(), "the sweep failed instead of skipping");
}

#[test]
fn a_named_per_game_step_selects_only_that_one() {
    let Ok(selected) =
        select_steps_including_games(Some("game.steam.proton"), Path::new(NO_USER_GAMES))
    else {
        // No Steam on this machine, which the test above covers.
        return;
    };

    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].id(),
        ImprovementId::from_static("game.steam.proton")
    );
}
