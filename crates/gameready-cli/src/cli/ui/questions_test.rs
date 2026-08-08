use std::collections::BTreeMap;
use std::path::PathBuf;

use gameready_core::games::{default_wrappers, AppId, GameProfile, Source};
use gameready_core::run::PreflightReport;
use gameready_core::steam::InstalledGame;
use gameready_core::steps::CpuGovernor;

use super::*;

fn setup(name: &str) -> GameSetup {
    let app_id = AppId(1_422_450);
    GameSetup {
        game: InstalledGame::new(app_id, name.to_owned(), PathBuf::from("/games")),
        source: Some(Source::Builtin),
        profile: GameProfile {
            name: name.to_owned(),
            app_id,
            wrappers: default_wrappers(),
            env: BTreeMap::new(),
            proton: None,
            override_module: None,
        },
    }
}

/// A plan with nothing pending, so a test can add only what it is about.
fn plan(pending: Vec<Box<dyn gameready_core::improvement::CoreImprovement>>) -> RunPlan {
    RunPlan {
        settled: Vec::new(),
        pending,
        deferred: Vec::new(),
        preflight: PreflightReport {
            dependencies: Vec::new(),
            total_install_bytes: 0,
        },
        step_installs: Vec::new(),
        step_present: Vec::new(),
        started: std::time::Instant::now(),
    }
}

fn asking<'a>(setups: &'a [GameSetup], plan: &'a RunPlan, mode: Mode) -> Questions<'a> {
    Questions {
        setups,
        plan,
        packages: PackageManagerKind::Apt,
        picker: Picker::Ask,
        overlay: None,
        mode,
        compat_tools: &[],
    }
}

#[test]
fn a_normal_run_with_games_has_the_three_questions_the_games_bring() {
    // Games, overlay, and what to do about Steam. Nothing to install and no
    // governor step, so those two do not count.
    let games = [setup("Deadlock")];
    let empty = plan(Vec::new());

    assert_eq!(asking(&games, &empty, Mode::Apply).count(), 3);
}

#[test]
fn a_machine_with_no_games_is_asked_nothing_about_them() {
    let empty = plan(Vec::new());

    assert_eq!(asking(&[], &empty, Mode::Apply).count(), 0);
}

#[test]
fn a_dry_run_only_asks_what_it_can_still_honour() {
    // Which games and whether to show the overlay both shape the plan it
    // prints. Closing Steam and installing packages are changes, so a dry run
    // has no business asking about either.
    let games = [setup("Deadlock")];
    let empty = plan(Vec::new());

    assert_eq!(asking(&games, &empty, Mode::DryRun).count(), 2);
}

#[test]
fn the_overlay_flag_takes_its_question_off_the_count() {
    let games = [setup("Deadlock")];
    let empty = plan(Vec::new());
    let mut questions = asking(&games, &empty, Mode::Apply);
    questions.overlay = Some(Overlay::Show);

    assert_eq!(questions.count(), 2);
}

#[test]
fn a_run_that_could_pin_the_governor_counts_that_question_too() {
    let games = [setup("Deadlock")];
    let governor = plan(vec![Box::new(CpuGovernor)]);

    assert_eq!(asking(&games, &governor, Mode::Apply).count(), 4);
}

#[test]
fn a_run_that_cannot_ask_counts_nothing() {
    let games = [setup("Deadlock")];
    let empty = plan(Vec::new());
    let mut questions = asking(&games, &empty, Mode::Apply);
    questions.picker = Picker::TakeAll;

    assert_eq!(questions.count(), 0);
}
