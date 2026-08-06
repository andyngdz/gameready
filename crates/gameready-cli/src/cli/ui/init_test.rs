use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{AppId, GameProfile, Source, Wrapper, default_wrappers};
use gameready_core::steam::{GameSetup, InstalledGame};

use super::LaunchInstructions;

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
fn a_game_with_a_profile_shows_its_launch_string() {
    let selected = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = LaunchInstructions::new(&selected).to_string();

    assert!(rendered.contains("gamemoderun %command%"), "{rendered}");
    assert!(rendered.contains("Deadlock"), "{rendered}");
}

#[test]
fn a_game_on_the_defaults_shows_its_launch_string_too() {
    let selected = [setup("Hollow Knight", 367_520, None)];
    let rendered = LaunchInstructions::new(&selected).to_string();

    assert!(rendered.contains("gamemoderun %command%"), "{rendered}");
    assert!(rendered.contains("Hollow Knight"), "{rendered}");
}

#[test]
fn nothing_is_printed_when_no_game_needs_launch_options() {
    // Reachable only through a profile that turns every wrapper off, now that a
    // game without a profile still gets the defaults.
    let selected = [setup("Hollow Knight", 367_520, Some(Vec::new()))];
    assert!(LaunchInstructions::new(&selected).is_empty());
    assert_eq!(LaunchInstructions::new(&selected).to_string(), "");
}
