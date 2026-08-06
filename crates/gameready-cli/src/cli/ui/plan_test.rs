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
fn only_selected_games_appear_in_the_plan() {
    let setups = [
        setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode])),
        setup("Hollow Knight", 367_520, None),
    ];
    let rendered = plan_for(&setups, Mode::Apply);

    assert!(rendered.contains("Deadlock"), "{rendered}");
    assert!(!rendered.contains("Hollow Knight"), "{rendered}");
}

#[test]
fn a_dry_run_says_nothing_will_change() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for(&setups, Mode::DryRun);

    assert!(rendered.contains("nothing will change"), "{rendered}");
}

#[test]
fn a_machine_with_no_games_says_so() {
    let rendered = plan_for(&[], Mode::Apply);
    assert!(rendered.contains("No games found"), "{rendered}");
}
