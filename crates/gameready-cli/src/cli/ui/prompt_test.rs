use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{AppId, GameProfile, Source, Wrapper, default_wrappers};
use gameready_core::steam::{GameSetup, InstalledGame};

use super::{choose_games, label};

fn setup(name: &str, has_profile: bool) -> GameSetup {
    let app_id = AppId(1);
    GameSetup {
        game: InstalledGame::new(app_id, name.to_owned(), PathBuf::from("/games")),
        profile: GameProfile {
            name: name.to_owned(),
            app_id,
            wrappers: if has_profile {
                vec![Wrapper::GameMode, Wrapper::Gamescope]
            } else {
                default_wrappers()
            },
            env: BTreeMap::new(),
            proton: None,
            override_module: None,
        },
        source: has_profile.then_some(Source::Builtin),
    }
}

#[test]
fn an_empty_library_never_opens_a_picker() {
    // prompt() on an empty list would block on a terminal with nothing to pick.
    assert!(choose_games(&[]).expect("no prompt").is_empty());
}

#[test]
fn a_game_with_a_profile_is_marked_as_tuned() {
    assert!(label(&setup("Deadlock", true)).contains("tuned profile"));
}

#[test]
fn a_game_without_a_profile_names_what_it_still_gets() {
    // Saying "no profile" would read as "gameready cannot help with this", when
    // in fact the game gets gamemode like every other one.
    assert!(label(&setup("Hollow Knight", false)).contains("gamemode"));
}

#[test]
fn the_label_leads_with_the_name_the_user_recognises() {
    assert!(label(&setup("Deadlock", true)).starts_with("Deadlock"));
}
