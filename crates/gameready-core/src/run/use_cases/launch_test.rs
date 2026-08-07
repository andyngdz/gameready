use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;
use crate::games::{default_wrappers, GameProfile, Source, Wrapper};
use crate::steam::InstalledGame;

fn setup(name: &str, app_id: u32, wrappers: Option<Vec<Wrapper>>) -> GameSetup {
    GameSetup {
        game: InstalledGame::new(AppId(app_id), name.to_owned(), PathBuf::from("/games")),
        source: wrappers.as_ref().map(|_| Source::Builtin),
        profile: GameProfile {
            name: name.to_owned(),
            app_id: AppId(app_id),
            wrappers: wrappers.unwrap_or_else(default_wrappers),
            env: BTreeMap::new(),
            proton: None,
            override_module: None,
        },
    }
}

#[test]
fn a_game_with_wrappers_becomes_a_target() {
    let targets = targets_for(&[setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))]);

    assert_eq!(targets.len(), 1);
    let deadlock = &targets[0];
    assert_eq!(deadlock.options, "gamemoderun %command%");
    assert_eq!(deadlock.name, "Deadlock");
}

#[test]
fn a_game_with_no_profile_becomes_a_target_on_the_defaults() {
    let targets = targets_for(&[setup("Hollow Knight", 367_520, None)]);

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].options, "gamemoderun %command%");
}

#[test]
fn a_profile_that_asks_for_nothing_is_left_out_rather_than_clearing_the_box() {
    // An empty value would wipe whatever the user typed into Steam themselves.
    assert!(targets_for(&[setup("Quiet", 1, Some(Vec::new()))]).is_empty());
}

#[test]
fn the_selection_order_is_kept() {
    let targets = targets_for(&[
        setup("Alpha", 1, Some(vec![Wrapper::GameMode])),
        setup("Beta", 2, Some(vec![Wrapper::MangoHud])),
    ]);

    let names: Vec<&str> = targets.iter().map(|target| target.name.as_str()).collect();
    assert_eq!(names, ["Alpha", "Beta"]);
}
