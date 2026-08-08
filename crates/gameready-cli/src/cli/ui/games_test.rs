use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{
    AppId, Catalog, GameError, GameProfile, ProtonChoice, Source, Wrapper,
};

use super::GameList;

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

#[test]
fn an_empty_catalog_says_so_rather_than_printing_a_bare_header() {
    let catalog = Catalog::new();
    let rendered = GameList::new(&catalog, &[]).to_string();
    assert!(rendered.contains("No game profiles"), "{rendered}");
}

#[test]
fn every_game_shows_its_appid_and_where_it_came_from() {
    let mut catalog = Catalog::new();
    catalog.overlay(Source::Builtin, [profile("Deadlock", 1422450, Vec::new())]);

    let rendered = GameList::new(&catalog, &[]).to_string();
    assert!(rendered.contains("Deadlock"), "{rendered}");
    assert!(rendered.contains("1422450"), "{rendered}");
    assert!(rendered.contains("built in"), "{rendered}");
}

#[test]
fn an_overridden_profile_is_marked_as_the_users_own() {
    // Otherwise a user who edited their copy has no way to confirm it is the
    // one in effect, which is the first thing to check when it "does nothing".
    let mut catalog = Catalog::new();
    catalog.overlay(Source::Builtin, [profile("Deadlock", 1422450, Vec::new())]);
    catalog.overlay(Source::User, [profile("Deadlock", 1422450, Vec::new())]);

    let rendered = GameList::new(&catalog, &[]).to_string();
    assert!(rendered.contains("yours"), "{rendered}");
    assert!(!rendered.contains("built in"), "{rendered}");
}

#[test]
fn wrappers_are_shown_as_the_commands_they_become() {
    let mut catalog = Catalog::new();
    catalog.overlay(
        Source::Builtin,
        [profile(
            "Deadlock",
            1422450,
            vec![Wrapper::GameMode, Wrapper::MangoHud],
        )],
    );

    let rendered = GameList::new(&catalog, &[]).to_string();
    assert!(rendered.contains("gamemoderun, mangohud"), "{rendered}");
}

#[test]
fn a_pinned_proton_version_is_listed_next_to_the_wrappers() {
    // A list that shows one setting and hides the other reads as if there is
    // only one, which is how the pin went unnoticed for as long as it did.
    let mut catalog = Catalog::new();
    let mut deadlock = profile("Deadlock", 1422450, vec![Wrapper::GameMode]);
    deadlock.proton = Some(ProtonChoice::Pinned {
        tool: "GE-Proton8-32".to_owned(),
    });
    catalog.overlay(Source::Builtin, [deadlock]);

    let rendered = GameList::new(&catalog, &[]).to_string();
    assert!(rendered.contains("GE-Proton8-32"), "{rendered}");
}

#[test]
fn asking_for_the_newest_build_says_so_rather_than_naming_one() {
    // Which build that is depends on what is installed, and the catalog is read
    // without touching a Steam directory.
    let mut catalog = Catalog::new();
    let mut deadlock = profile("Deadlock", 1422450, vec![Wrapper::GameMode]);
    deadlock.proton = Some(ProtonChoice::NewestGeProton);
    catalog.overlay(Source::Builtin, [deadlock]);

    let rendered = GameList::new(&catalog, &[]).to_string();
    assert!(rendered.contains("newest GE-Proton"), "{rendered}");
}

#[test]
fn a_profile_that_failed_to_load_is_reported_under_its_own_heading() {
    let catalog = Catalog::new();
    let failures = [GameError::NoName {
        path: PathBuf::from("/home/someone/.config/gameready/games/Foo/game.toml"),
    }];

    let rendered = GameList::new(&catalog, &failures).to_string();
    assert!(rendered.contains("Couldn't read 1 file"), "{rendered}");
    assert!(rendered.contains("Foo/game.toml"), "{rendered}");
}
