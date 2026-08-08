use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{default_wrappers, AppId, GameProfile, Source, Wrapper};
use gameready_core::run::{targets_for, InstallConsent};
use gameready_core::steam::{GameSetup, InstalledGame};
use gameready_core::steps::CompatTarget;

use super::LaunchInstructions;
use crate::cli::ui::{Answers, LaunchChoice};

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

fn answers(selected: Vec<GameSetup>, proton: Vec<CompatTarget>) -> Answers {
    Answers {
        targets: targets_for(&selected),
        selected,
        proton,
        launch: LaunchChoice::ShowForCopying,
        consent: InstallConsent::Declined,
        overlay: gameready_core::steam::Overlay::Hide,
        governor_pinned: false,
    }
}

#[test]
fn a_game_with_a_profile_shows_its_launch_string() {
    let selected = vec![setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = LaunchInstructions::new(&answers(selected, Vec::new())).to_string();

    assert!(rendered.contains("gamemoderun %command%"), "{rendered}");
    assert!(rendered.contains("Deadlock"), "{rendered}");
}

#[test]
fn a_game_on_the_defaults_shows_its_launch_string_too() {
    let selected = vec![setup("Hollow Knight", 367_520, None)];
    let rendered = LaunchInstructions::new(&answers(selected, Vec::new())).to_string();

    assert!(rendered.contains("gamemoderun %command%"), "{rendered}");
    assert!(rendered.contains("Hollow Knight"), "{rendered}");
}

#[test]
fn a_pinned_game_is_told_which_proton_build_to_pick() {
    // The manual path has to carry every setting the writing path would have
    // made, or the user who declined loses one of them without being told.
    let selected = vec![setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let proton = vec![CompatTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        tool: "GE-Proton11-3".to_owned(),
    }];
    let rendered = LaunchInstructions::new(&answers(selected, proton)).to_string();

    assert!(rendered.contains("Compatibility"), "{rendered}");
    assert!(rendered.contains("GE-Proton11-3"), "{rendered}");
}

#[test]
fn a_game_that_only_needs_a_pin_is_still_listed() {
    let selected = vec![setup("Deadlock", 1_422_450, Some(Vec::new()))];
    let proton = vec![CompatTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        tool: "GE-Proton11-3".to_owned(),
    }];
    let rendered = LaunchInstructions::new(&answers(selected, proton)).to_string();

    assert!(rendered.contains("Deadlock"), "{rendered}");
    assert!(!rendered.contains("Launch Options"), "{rendered}");
}

#[test]
fn nothing_is_printed_when_no_game_needs_anything() {
    // Reachable only through a profile that turns every wrapper off, now that a
    // game without a profile still gets the defaults.
    let selected = vec![setup("Hollow Knight", 367_520, Some(Vec::new()))];
    let answers = answers(selected, Vec::new());

    assert!(LaunchInstructions::new(&answers).is_empty());
    assert_eq!(LaunchInstructions::new(&answers).to_string(), "");
}
