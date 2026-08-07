use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{default_wrappers, AppId, GameProfile, Source, Wrapper};
use gameready_core::steam::InstalledGame;

use gameready_core::facts::PackageManagerKind;
use gameready_core::run::{PreflightReport, RunPlan};

use super::*;
use crate::cli::ui::{ask_everything, Picker, Questions};

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

fn plan_for(setups: &[GameSetup], mode: Mode) -> String {
    plan_for_selection(setups, setups, mode)
}

/// Renders the plan for a run that found `found` and picked `selected`.
///
/// The two lists are separate because the plan's job is to show the second one,
/// and a helper that could only pass the same list twice could not tell whether
/// it did.
fn plan_for_selection(found: &[GameSetup], selected: &[GameSetup], mode: Mode) -> String {
    let plan = RunPlan {
        settled: Vec::new(),
        pending: Vec::new(),
        preflight: PreflightReport {
            dependencies: Vec::new(),
            total_install_bytes: 0,
        },
        step_installs: Vec::new(),
        step_present: Vec::new(),
        started: std::time::Instant::now(),
    };
    let answers = ask_everything(&Questions {
        setups: selected,
        plan: &plan,
        packages: PackageManagerKind::Apt,
        picker: Picker::TakeAll,
        overlay: None,
        mode,
        compat_tools: &[],
    })
    .expect("answered");
    InitPlan::new(found, &answers, mode).to_string()
}

#[test]
fn only_selected_games_appear_in_the_plan() {
    let found = [
        setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode])),
        setup("Hollow Knight", 367_520, None),
    ];
    let rendered = plan_for_selection(&found, &found[..1], Mode::Apply);

    assert!(rendered.contains("Deadlock"), "{rendered}");
    assert!(!rendered.contains("Hollow Knight"), "{rendered}");
}

#[test]
fn a_game_on_the_defaults_appears_once_it_is_picked() {
    // It has no profile, but it is getting gamemode, so leaving it out of the
    // plan would hide a change the run is about to make.
    let found = [setup("Hollow Knight", 367_520, None)];
    let rendered = plan_for(&found, Mode::Apply);

    assert!(rendered.contains("Hollow Knight"), "{rendered}");
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
