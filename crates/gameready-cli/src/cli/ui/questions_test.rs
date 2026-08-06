use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{AppId, GameProfile, Source, Wrapper};
use gameready_core::steam::InstalledGame;

use super::*;

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
fn a_scripted_run_answers_everything_without_prompting() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = ask_everything(&setups, Picker::TakeAll, None, Mode::Apply).expect("answered");

    assert_eq!(answers.selected.len(), 1);
    assert_eq!(answers.targets.len(), 1);
    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn a_scripted_run_never_closes_steam() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = ask_everything(&setups, Picker::TakeAll, None, Mode::Apply).expect("answered");

    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn the_overlay_flag_is_honoured_without_a_prompt() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = ask_everything(&setups, Picker::TakeAll, Some(Overlay::Show), Mode::Apply)
        .expect("answered");

    assert_eq!(answers.targets[0].options, "gamemoderun mangohud %command%");
}

#[test]
fn a_game_with_no_profile_produces_no_launch_target() {
    let setups = [setup("Hollow Knight", 367_520, None)];
    let answers = ask_everything(&setups, Picker::TakeAll, None, Mode::Apply).expect("answered");

    assert!(answers.targets.is_empty());
    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn a_dry_run_asks_nothing_that_would_change_the_machine() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = ask_everything(&setups, Picker::TakeAll, None, Mode::DryRun).expect("answered");

    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn nothing_to_apply_means_nothing_to_ask_about_applying_it() {
    let answers = ask_everything(&[], Picker::TakeAll, None, Mode::Apply).expect("answered");

    assert!(answers.selected.is_empty());
    assert!(answers.targets.is_empty());
}
