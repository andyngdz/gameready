use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{AppId, GameProfile, Source};
use gameready_core::steam::{GameSetup, InstalledGame};

use super::{choose_games, label};

fn setup(name: &str, has_profile: bool) -> GameSetup {
    let app_id = AppId(1);
    GameSetup {
        game: InstalledGame::new(app_id, name.to_owned(), PathBuf::from("/games")),
        profile: has_profile.then(|| GameProfile {
            name: name.to_owned(),
            app_id,
            wrappers: Vec::new(),
            env: BTreeMap::new(),
            proton: None,
            override_module: None,
        }),
        source: has_profile.then_some(Source::Builtin),
    }
}

#[test]
fn an_empty_library_never_opens_a_picker() {
    // prompt() on an empty list would block on a terminal with nothing to pick.
    assert!(choose_games(&[]).expect("no prompt").is_empty());
}

#[test]
fn a_game_with_a_profile_is_marked_as_having_one() {
    assert!(label(&setup("Deadlock", true)).contains("has a profile"));
}

#[test]
fn a_game_without_a_profile_says_what_it_still_gets() {
    // "no profile" alone would read as "gameready cannot help with this".
    assert!(label(&setup("Hollow Knight", false)).contains("core tuning only"));
}

#[test]
fn the_label_leads_with_the_name_the_user_recognises() {
    assert!(label(&setup("Deadlock", true)).starts_with("Deadlock"));
}
