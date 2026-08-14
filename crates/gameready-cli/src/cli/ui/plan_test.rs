use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{default_wrappers, AppId, GameProfile, Source, Wrapper};
use gameready_core::improvement::CoreImprovement;
use gameready_core::steam::InstalledGame;
use gameready_core::steps::{CpuGovernor, MaxMapCount};

use gameready_core::facts::PackageManagerKind;
use gameready_core::run::PreflightReport;

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

fn run_plan(pending: Vec<Box<dyn CoreImprovement>>) -> RunPlan {
    RunPlan {
        settled: Vec::new(),
        pending,
        deferred: Vec::new(),
        contested: Vec::new(),
        preflight: PreflightReport {
            dependencies: Vec::new(),
            total_install_bytes: 0,
        },
        step_installs: Vec::new(),
        step_present: Vec::new(),
        started: std::time::Instant::now(),
    }
}

fn plan_for(setups: &[GameSetup], mode: Mode) -> String {
    plan_for_selection(setups, setups, mode, Vec::new())
}

/// Renders the plan for a run that found `found` and picked `selected`.
///
/// The two lists are separate because the plan's job is to show the second one,
/// and a helper that could only pass the same list twice could not tell whether
/// it did.
fn plan_for_selection(
    found: &[GameSetup],
    selected: &[GameSetup],
    mode: Mode,
    pending: Vec<Box<dyn CoreImprovement>>,
) -> String {
    let plan = run_plan(pending);
    let answers = ask_everything(&Questions {
        setups: selected,
        plan: &plan,
        packages: PackageManagerKind::Apt,
        picker: Picker::TakeAll,
        overlay: None,
        mode,
    })
    .expect("answered");
    let rendered = InitPlan::new(found, &answers, &plan, PackageManagerKind::Apt, mode).to_string();
    console::strip_ansi_codes(&rendered).into_owned()
}

#[test]
fn only_selected_games_appear_in_the_plan() {
    let found = [
        setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode])),
        setup("Hollow Knight", 367_520, None),
    ];
    let rendered = plan_for_selection(&found, &found[..1], Mode::Apply, Vec::new());

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

    assert!(
        rendered.contains("nothing below actually happens"),
        "{rendered}"
    );
}

#[test]
fn a_machine_with_no_games_says_so() {
    let rendered = plan_for(&[], Mode::Apply);
    assert!(rendered.contains("no games found"), "{rendered}");
}

#[test]
fn the_per_game_row_counts_each_kind_of_setting_separately() {
    // Launch options and a pinned Proton build are different promises, and a
    // single "3 settings" would let either one arrive as a surprise.
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for(&setups, Mode::Apply);

    assert!(rendered.contains("launch options ×1"), "{rendered}");
}

#[test]
fn the_system_row_counts_the_tunings_and_names_what_they_are_about() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for_selection(
        &setups,
        &setups,
        Mode::Apply,
        vec![Box::new(MaxMapCount), Box::new(CpuGovernor)],
    );

    assert!(rendered.contains("2 tunings"), "{rendered}");
    assert!(rendered.contains("memory"), "{rendered}");
}

#[test]
fn a_run_of_one_tuning_is_not_reported_as_one_tunings() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for_selection(&setups, &setups, Mode::Apply, vec![Box::new(MaxMapCount)]);

    assert!(rendered.contains("1 tuning "), "{rendered}");
}

#[test]
fn the_plan_ends_on_the_command_that_takes_it_all_back() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for(&setups, Mode::Apply);

    assert!(rendered.contains("gameready rollback"), "{rendered}");
}

#[test]
fn a_run_that_touches_nothing_outside_home_does_not_promise_a_password_prompt() {
    // Only Steam config is being written, and that never asks for a password.
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for(&setups, Mode::Apply);

    assert!(!rendered.contains("password"), "{rendered}");
}

#[test]
fn a_run_with_a_root_step_says_the_password_is_asked_once() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let rendered = plan_for_selection(&setups, &setups, Mode::Apply, vec![Box::new(MaxMapCount)]);

    assert!(rendered.contains("password once"), "{rendered}");
}
