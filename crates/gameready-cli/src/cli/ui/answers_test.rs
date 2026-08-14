use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::facts::PackageManagerKind;
use gameready_core::games::{default_wrappers, AppId, GameProfile, ProtonChoice, Source, Wrapper};
use gameready_core::run::{Mode, PreflightReport, RunPlan};
use gameready_core::steam::{InstalledGame, Overlay};
use gameready_core::steps::CompatRank;

use super::*;
use crate::cli::ui::{Picker, Questions};

fn setup(name: &str, app_id: u32, wrappers: Option<Vec<Wrapper>>) -> GameSetup {
    with_proton(name, app_id, wrappers, None)
}

fn with_proton(
    name: &str,
    app_id: u32,
    wrappers: Option<Vec<Wrapper>>,
    proton: Option<ProtonChoice>,
) -> GameSetup {
    GameSetup {
        game: InstalledGame::new(AppId(app_id), name.to_owned(), PathBuf::from("/games")),
        source: wrappers.as_ref().map(|_| Source::Builtin),
        profile: GameProfile {
            name: name.to_owned(),
            app_id: AppId(app_id),
            wrappers: wrappers.unwrap_or_else(default_wrappers),
            env: BTreeMap::new(),
            proton,
            override_module: None,
        },
    }
}

/// A plan with nothing to do, so these tests exercise the questions rather than
/// the run behind them.
fn empty_plan() -> RunPlan {
    RunPlan {
        settled: Vec::new(),
        pending: Vec::new(),
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

/// Asks with a scripted picker, which answers without prompting.
fn scripted(setups: &[GameSetup], overlay: Option<Overlay>, mode: Mode) -> Answers {
    let plan = empty_plan();
    ask_everything(&Questions {
        setups,
        plan: &plan,
        packages: PackageManagerKind::Apt,
        picker: Picker::TakeAll,
        overlay,
        mode,
    })
    .expect("answered")
}

#[test]
fn a_scripted_run_answers_everything_without_prompting() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = scripted(&setups, None, Mode::Apply);

    assert_eq!(answers.selected.len(), 1);
    assert_eq!(answers.targets.len(), 1);
    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn a_scripted_run_never_closes_steam() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = scripted(&setups, None, Mode::Apply);

    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn the_overlay_flag_is_honoured_without_a_prompt() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = scripted(&setups, Some(Overlay::Show), Mode::Apply);

    assert_eq!(answers.targets[0].options, "gamemoderun mangohud %command%");
}

#[test]
fn a_game_with_no_profile_still_produces_a_launch_target() {
    let setups = [setup("Hollow Knight", 367_520, None)];
    let answers = scripted(&setups, None, Mode::Apply);

    assert_eq!(answers.targets.len(), 1);
    assert_eq!(answers.targets[0].options, "gamemoderun %command%");
}

#[test]
fn a_profile_that_turns_everything_off_produces_no_launch_target() {
    let setups = [setup("Hollow Knight", 367_520, Some(Vec::new()))];
    let answers = scripted(&setups, None, Mode::Apply);

    assert!(answers.targets.is_empty());
    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn a_dry_run_asks_nothing_that_would_change_the_machine() {
    let setups = [setup("Deadlock", 1_422_450, Some(vec![Wrapper::GameMode]))];
    let answers = scripted(&setups, None, Mode::DryRun);

    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
    assert_eq!(answers.launch, LaunchChoice::ShowForCopying);
}

#[test]
fn nothing_to_apply_means_nothing_to_ask_about_applying_it() {
    let answers = scripted(&[], None, Mode::Apply);

    assert!(answers.selected.is_empty());
    assert!(answers.targets.is_empty());
    assert!(answers.proton.is_empty());
}

#[test]
fn a_profile_asking_for_ge_proton_becomes_a_wish_that_names_no_build() {
    // Nothing here reads compatibilitytools.d. Which build the wish resolves to
    // is settled after the run has installed one, not while it is answering.
    let setups = [with_proton(
        "Deadlock",
        1_422_450,
        Some(vec![Wrapper::GameMode]),
        Some(ProtonChoice::NewestGeProton),
    )];
    let answers = scripted(&setups, None, Mode::Apply);

    assert_eq!(answers.proton[0].choice, ProtonChoice::NewestGeProton);
    assert_eq!(answers.proton[0].app_id, AppId(1_422_450));
}

#[test]
fn a_scripted_run_takes_the_machine_wide_default_along_with_the_games() {
    let setups = [with_proton(
        "Deadlock",
        1_422_450,
        Some(vec![Wrapper::GameMode]),
        Some(ProtonChoice::NewestGeProton),
    )];
    let answers = scripted(&setups, None, Mode::Apply);

    assert_eq!(answers.proton.len(), 2);
    assert_eq!(answers.proton[1].rank, CompatRank::MachineWide);
}

#[test]
fn no_game_asking_for_a_build_leaves_steams_proton_settings_untouched() {
    // The machine-wide default rides along with the games. With no game asking
    // there is nothing to ask the user about, so nothing is written at all.
    let setups = [setup(
        "Hollow Knight",
        367_520,
        Some(vec![Wrapper::GameMode]),
    )];
    let answers = scripted(&setups, None, Mode::Apply);

    assert!(answers.proton.is_empty());
    // The launch options still go through: one setting standing down does not
    // take the other with it.
    assert_eq!(answers.targets.len(), 1);
}
