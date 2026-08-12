use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;
use crate::games::{AppId, GameProfile, ProtonChoice, Wrapper};
use crate::steam::InstalledGame;

fn setup(name: &str, app_id: u32, proton: Option<ProtonChoice>) -> GameSetup {
    GameSetup {
        game: InstalledGame::new(AppId(app_id), name.to_owned(), PathBuf::from(name)),
        profile: GameProfile {
            name: name.to_owned(),
            app_id: AppId(app_id),
            wrappers: vec![Wrapper::GameMode],
            env: BTreeMap::new(),
            proton,
            override_module: None,
        },
        source: None,
    }
}

#[test]
fn a_profile_asking_for_ge_proton_becomes_a_wish_that_still_says_so() {
    // The wish carries the profile's own words. Which build that is depends on
    // what is installed when the run gets round to writing it, and that is not
    // known here.
    let wishes = compat_wishes_for(&[setup(
        "Deadlock",
        1_422_450,
        Some(ProtonChoice::NewestGeProton),
    )]);

    assert_eq!(wishes.len(), 1);
    let deadlock = &wishes[0];
    assert_eq!(deadlock.choice, ProtonChoice::NewestGeProton);
    assert_eq!(deadlock.name, "Deadlock");
    assert_eq!(deadlock.app_id, AppId(1_422_450));
    assert_eq!(deadlock.rank, CompatRank::Game);
}

#[test]
fn a_profile_that_says_nothing_about_proton_wishes_for_nothing() {
    // Steam's own choice is the default, and overwriting it for a game nobody
    // said anything about is a change nobody asked for.
    let wishes = compat_wishes_for(&[setup("Hollow Knight", 367_520, None)]);

    assert!(wishes.is_empty());
}

#[test]
fn every_game_with_a_choice_gets_its_own_wish() {
    let wishes = compat_wishes_for(&[
        setup("Deadlock", 1_422_450, Some(ProtonChoice::NewestGeProton)),
        setup(
            "Cyberpunk 2077",
            1_091_500,
            Some(ProtonChoice::Experimental),
        ),
        setup("Hollow Knight", 367_520, None),
    ]);

    assert_eq!(wishes.len(), 2);
    assert_eq!(wishes[1].choice, ProtonChoice::Experimental);
}
