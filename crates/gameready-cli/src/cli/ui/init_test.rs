use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{AppId, GameProfile, Source, Wrapper};
use gameready_core::steam::{GameSetup, InstalledGame};

use super::LaunchInstructions;

fn setup(name: &str, app_id: u32, wrappers: Option<Vec<Wrapper>>) -> GameSetup {
    GameSetup {
        game: InstalledGame::new(AppId(app_id), name.to_owned(), PathBuf::from("/games")),
        profile: wrappers.map(|wrappers| GameProfile {
            name: name.to_owned(),
            app_id: AppId(app_id),
            wrappers,
            env: BTreeMap::new(),
            proton: None,
            override_module: None,
        }),
        source: Some(Source::Builtin),
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
fn nothing_is_printed_when_no_game_needs_launch_options() {
    let selected = [setup("Hollow Knight", 367_520, None)];
    assert!(LaunchInstructions::new(&selected).is_empty());
    assert_eq!(LaunchInstructions::new(&selected).to_string(), "");
}
