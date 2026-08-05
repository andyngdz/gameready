use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{AppId, GameProfile, Source, Wrapper};
use gameready_core::steam::InstalledGame;

use super::*;
use crate::cli::ui::{Picker, ask_everything};

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

fn plan_for(setups: &[GameSetup], mode: Mode) -> String {
    let answers = ask_everything(setups, Picker::TakeAll, None, mode).expect("answered");
    InitPlan::new(setups, &answers, mode).to_string()
}

#[test]
fn a_chosen_game_is_marked_and_an_unchosen_one_is_not() {
    let setups = [
        setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode])),
        setup("Hollow Knight", 367_520, None),
    ];
    let rendered = plan_for(&setups, Mode::Apply);

    assert!(rendered.contains("* Deadlock"), "{rendered}");
    assert!(rendered.contains("  Hollow Knight"), "{rendered}");
}

#[test]
fn a_dry_run_says_up_front_that_nothing_will_change() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for(&setups, Mode::DryRun);

    assert!(
        rendered.contains("nothing below will actually change"),
        "{rendered}"
    );
}

#[test]
fn a_run_that_will_not_close_steam_does_not_threaten_to() {
    // Picker::TakeAll never closes Steam, so saying it would is a lie the user
    // would act on by quitting their game first for no reason.
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for(&setups, Mode::Apply);

    assert!(!rendered.contains("Steam will be closed"), "{rendered}");
}

#[test]
fn a_machine_with_no_games_says_none() {
    assert!(plan_for(&[], Mode::Apply).contains("none"));
}
