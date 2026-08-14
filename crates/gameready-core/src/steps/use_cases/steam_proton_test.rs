use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::games::AppId;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steam::PriorBlock;
use crate::steps::CompatRank;

const CONFIG: &str = "/home/someone/.steam/steam/config/config.vdf";

fn config_text() -> String {
    indoc! {r#"
        "InstallConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "CompatToolMapping"
                        {
                            "0"
                            {
                                "name"        "GE-Proton11-3"
                                "config"        ""
                                "priority"        "75"
                            }
                        }
                    }
                }
            }
        }
    "#}
    .to_owned()
}

fn target(tool: &str) -> CompatTarget {
    CompatTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        tool: tool.to_owned(),
        rank: CompatRank::Game,
    }
}

fn step(tool: &str) -> SteamProton {
    SteamProton::new(PathBuf::from(CONFIG), vec![target(tool)])
}

fn machine() -> MockRunner {
    MockRunner::new().with_file(CONFIG, config_text())
}

#[test]
fn probe_is_applicable_when_the_game_has_no_entry_yet() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    assert!(matches!(
        step("GE-Proton11-3").probe(&cx).expect("probed"),
        Probe::Applicable
    ));
}

#[test]
fn probe_is_already_applied_once_the_entry_says_the_same_build() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    applied(&runner, &dir, "GE-Proton11-3");

    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let probe = step("GE-Proton11-3").probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }), "{probe:?}");
}

#[test]
fn probe_is_not_applicable_with_no_game_asking_for_a_version() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let empty = SteamProton::new(PathBuf::from(CONFIG), Vec::new());

    let probe = empty.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn probe_is_not_applicable_without_a_config_file() {
    let runner = MockRunner::new();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let probe = step("GE-Proton11-3").probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn the_plan_names_the_game_and_the_build() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let plan = step("GE-Proton11-3").plan(&cx).expect("planned");
    let rendered = format!("{:?}", plan.actions);
    assert!(rendered.contains("Deadlock"), "{rendered}");
    assert!(rendered.contains("GE-Proton11-3"), "{rendered}");
}

fn applied(runner: &MockRunner, dir: &TempDir, tool: &str) -> Vec<Change> {
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, runner);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, SteamProton::id_const(), runner, &mut journal);

    step(tool).apply(&mut apply).expect("applied");
    apply.recorded().to_vec()
}

/// Reverses a recorded apply through the step's own rollback.
fn roll_back(runner: &MockRunner, dir: &TempDir, recorded: &[Change]) {
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, runner);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, SteamProton::id_const(), runner, &mut journal);

    step("GE-Proton11-3")
        .rollback(recorded, &mut apply)
        .expect("rolled back");
}

#[test]
fn apply_writes_the_config_and_journals_that_the_entry_was_ours() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    let recorded = applied(&runner, &dir, "GE-Proton11-3");

    match recorded.as_slice() {
        [Change::SteamConfigWritten { sections, .. }] => {
            assert_eq!(sections.len(), 1);
            // The game had no compatibility entry, so undoing removes the one
            // gameready added rather than setting its keys to empty.
            assert_eq!(sections[0].prior, PriorBlock::Absent);
        }
        other => panic!("expected one Steam config write, got {other:?}"),
    }
    assert!(runner.file(CONFIG).expect("config").contains("1422450"));
}

#[test]
fn apply_keeps_no_copy_of_a_file_holding_credentials() {
    // config.vdf carries the account's stored credentials. Nothing reads a
    // pre-image any more, so keeping one in a directory nothing prunes would be
    // a copy of those for no purpose.
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    applied(&runner, &dir, "GE-Proton11-3");

    let copies: Vec<_> = runner
        .paths()
        .into_iter()
        .filter(|path| path != CONFIG)
        .collect();
    assert!(copies.is_empty(), "a pre-image was written: {copies:?}");
}

#[test]
fn the_machine_wide_default_survives_a_per_game_pin() {
    // The appid 0 entry is Steam's "run everything through this" setting, and
    // taking it away would change every other game on the machine.
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    applied(&runner, &dir, "GE-Proton11-3");

    let written = runner.file(CONFIG).expect("config");
    assert!(written.contains("\"75\""), "{written}");
}

#[test]
fn rollback_takes_the_entry_gameready_added_back_out() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorded = applied(&runner, &dir, "GE-Proton11-3");

    roll_back(&runner, &dir, &recorded);

    let after = runner.file(CONFIG).expect("still there");
    assert!(!after.contains("1422450"), "{after}");
    // The machine-wide entry is Steam's, not part of what the run added.
    assert!(after.contains("\"75\""), "{after}");
}

#[test]
fn rollback_keeps_what_steam_wrote_after_the_run() {
    // Steam saves config.vdf every time it exits. A pre-image restore would
    // take away whatever it recorded between the run and the rollback.
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorded = applied(&runner, &dir, "GE-Proton11-3");

    let written = runner.file(CONFIG).expect("written");
    let steam_wrote = written.replace(
        "\"CompatToolMapping\"",
        "\"SurveyDateVersion\"\t\t\"1\"\n\t\t\t\t\t\"CompatToolMapping\"",
    );
    let runner = MockRunner::new().with_file(CONFIG, &steam_wrote);

    roll_back(&runner, &dir, &recorded);

    let after = runner.file(CONFIG).expect("still there");
    assert!(after.contains("SurveyDateVersion"), "{after}");
    assert!(!after.contains("1422450"), "{after}");
}

#[test]
fn verify_fails_while_the_game_is_still_unpinned() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let verification = step("GE-Proton11-3").verify(&cx).expect("verified");
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn verify_passes_once_the_config_says_what_was_asked_for() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    applied(&runner, &dir, "GE-Proton11-3");

    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    assert!(step("GE-Proton11-3")
        .verify(&cx)
        .expect("verified")
        .passed());
}
