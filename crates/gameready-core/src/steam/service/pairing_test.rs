use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;
use crate::games::{AppId, Wrapper};

fn setup(name: &str, app_id: u32, wrappers: Option<Vec<Wrapper>>) -> GameSetup {
    GameSetup {
        game: installed(name, app_id),
        profile: wrappers.map(|wrappers| profile(name, app_id, wrappers)),
        source: Some(Source::Builtin),
    }
}

fn profile(name: &str, app_id: u32, wrappers: Vec<Wrapper>) -> GameProfile {
    GameProfile {
        name: name.to_owned(),
        app_id: AppId(app_id),
        wrappers,
        env: BTreeMap::new(),
        proton: None,
        override_module: None,
    }
}

fn installed(name: &str, app_id: u32) -> InstalledGame {
    InstalledGame::new(AppId(app_id), name.to_owned(), PathBuf::from("/games"))
}

fn catalog_with(profiles: Vec<GameProfile>) -> Catalog {
    let mut catalog = Catalog::new();
    catalog.overlay(Source::Builtin, profiles);
    catalog
}

#[test]
fn a_game_with_a_profile_carries_it() {
    let catalog = catalog_with(vec![profile(
        "Deadlock",
        1_422_450,
        vec![Wrapper::GameMode],
    )]);
    let setups = pair_with_catalog(&[installed("Deadlock", 1_422_450)], &catalog);

    assert!(setups[0].has_profile());
    assert_eq!(setups[0].launch_options(), "gamemoderun %command%");
}

#[test]
fn a_game_with_no_profile_is_kept_in_the_list() {
    // The core tuning applies to every game, so dropping it would suggest
    // gameready cannot help with it at all.
    let catalog = catalog_with(Vec::new());
    let setups = pair_with_catalog(&[installed("Hollow Knight", 367_520)], &catalog);

    assert_eq!(setups.len(), 1);
    assert!(!setups[0].has_profile());
    assert_eq!(setups[0].launch_options(), "");
}

#[test]
fn matching_is_by_appid_not_by_name() {
    // Steam's name and the profile's name drift apart over re-releases, and the
    // appid is what identifies the thing being tuned.
    let catalog = catalog_with(vec![profile(
        "Deadlock",
        1_422_450,
        vec![Wrapper::GameMode],
    )]);
    let setups = pair_with_catalog(&[installed("Deadlock Playtest", 1_422_450)], &catalog);

    assert!(setups[0].has_profile());
}

#[test]
fn a_name_that_matches_but_an_appid_that_does_not_is_no_match() {
    let catalog = catalog_with(vec![profile("Deadlock", 1_422_450, Vec::new())]);
    let setups = pair_with_catalog(&[installed("Deadlock", 9_999_999)], &catalog);

    assert!(!setups[0].has_profile());
}

#[test]
fn the_source_of_the_winning_profile_comes_through() {
    let mut catalog = catalog_with(vec![profile("Deadlock", 1_422_450, Vec::new())]);
    catalog.overlay(Source::User, [profile("Deadlock", 1_422_450, Vec::new())]);

    let setups = pair_with_catalog(&[installed("Deadlock", 1_422_450)], &catalog);
    assert_eq!(setups[0].source, Some(Source::User));
}

#[test]
fn the_scan_order_is_preserved() {
    let catalog = catalog_with(Vec::new());
    let setups = pair_with_catalog(
        &[
            installed("Alpha", 1),
            installed("Beta", 2),
            installed("Gamma", 3),
        ],
        &catalog,
    );

    let names: Vec<&str> = setups
        .iter()
        .map(|setup| setup.game.name.as_str())
        .collect();
    assert_eq!(names, ["Alpha", "Beta", "Gamma"]);
}

#[test]
fn hiding_the_overlay_leaves_every_profile_as_it_was() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    assert_eq!(with_overlay(&setups, Overlay::Hide), setups.to_vec());
}

#[test]
fn showing_the_overlay_adds_mangohud_innermost() {
    // gamemode has to wrap it, not the other way round.
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let shown = with_overlay(&setups, Overlay::Show);

    assert_eq!(shown[0].launch_options(), "gamemoderun mangohud %command%");
}

#[test]
fn a_profile_that_already_asks_for_the_overlay_does_not_get_it_twice() {
    let setups = [setup(
        "Deadlock",
        1_422_450,
        Some(vec![Wrapper::GameMode, Wrapper::MangoHud]),
    )];
    let shown = with_overlay(&setups, Overlay::Show);

    assert_eq!(shown[0].launch_options(), "gamemoderun mangohud %command%");
}

#[test]
fn a_game_with_no_profile_is_untouched_by_the_overlay() {
    // Its launch options are never written, so there is nothing to add to.
    let setups = [setup("Hollow Knight", 367_520, None)];
    let shown = with_overlay(&setups, Overlay::Show);

    assert!(!shown[0].has_profile());
    assert_eq!(shown[0].launch_options(), "");
}

#[test]
fn the_unanswered_question_means_no_overlay() {
    // Putting an overlay on someone's screen is not something to do because
    // nobody said otherwise.
    assert_eq!(Overlay::default_answer(), Overlay::Hide);
}
