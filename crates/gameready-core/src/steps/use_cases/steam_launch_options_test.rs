use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::games::AppId;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};

const CONFIG: &str = "/home/someone/.steam/steam/userdata/1/config/localconfig.vdf";

fn config_text() -> String {
    indoc! {r#"
        "UserLocalConfigStore"
        {
        	"Software"
        	{
        		"Valve"
        		{
        			"Steam"
        			{
        				"apps"
        				{
        					"1422450"
        					{
        						"LaunchOptions"		"LD_PRELOAD="
        					}
        				}
        			}
        		}
        	}
        }
    "#}
    .to_owned()
}

fn target(options: &str) -> LaunchTarget {
    LaunchTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        options: options.to_owned(),
    }
}

fn step(options: &str) -> SteamLaunchOptions {
    SteamLaunchOptions::new(PathBuf::from(CONFIG), vec![target(options)])
}

fn machine() -> MockRunner {
    MockRunner::new().with_file(CONFIG, config_text())
}

#[test]
fn probe_is_applicable_when_the_options_differ() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    assert!(matches!(
        step("gamemoderun %command%").probe(&cx).expect("probed"),
        Probe::Applicable
    ));
}

#[test]
fn probe_is_already_applied_when_they_match() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let probe = step("LD_PRELOAD=").probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }), "{probe:?}");
}

#[test]
fn probe_is_not_applicable_with_no_games_selected() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let empty = SteamLaunchOptions::new(PathBuf::from(CONFIG), Vec::new());

    let probe = empty.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn probe_is_not_applicable_without_a_config_file() {
    let runner = MockRunner::new();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let probe = step("gamemoderun %command%").probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn the_plan_names_the_value_it_would_replace() {
    // The user typed LD_PRELOAD= themselves; replacing it without saying so
    // would take away a setting they made on purpose.
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let plan = step("gamemoderun %command%").plan(&cx).expect("planned");
    let rendered = format!("{:?}", plan.actions);
    assert!(rendered.contains("LD_PRELOAD="), "{rendered}");
    assert!(rendered.contains("Deadlock"), "{rendered}");
}

fn applied(runner: &MockRunner, dir: &TempDir, options: &str) -> Vec<Change> {
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, runner);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, SteamLaunchOptions::id_const(), runner, &mut journal);

    step(options).apply(&mut apply).expect("applied");
    apply.recorded().to_vec()
}

#[test]
fn apply_writes_the_config_and_journals_a_backup() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    let recorded = applied(&runner, &dir, "gamemoderun %command%");

    match recorded.as_slice() {
        [
            Change::FileWritten {
                backup, existed, ..
            },
        ] => {
            assert!(existed, "the config already existed");
            assert!(backup.is_some(), "no pre-image was recorded");
        }
        other => panic!("expected one file write, got {other:?}"),
    }
    assert!(
        runner
            .file(CONFIG)
            .expect("config")
            .contains("gamemoderun %command%")
    );
}

#[test]
fn the_backup_holds_the_file_exactly_as_it_was() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    let recorded = applied(&runner, &dir, "gamemoderun %command%");
    let Some(Change::FileWritten {
        backup: Some(backup),
        ..
    }) = recorded.first()
    else {
        panic!("no backup recorded");
    };

    assert_eq!(runner.file(backup).as_deref(), Some(config_text().as_str()));
}

#[test]
fn rollback_puts_the_original_config_back_byte_for_byte() {
    // This file holds every game's playtime and cloud sync state, so a rollback
    // that restored only the one value would not put a mistake right.
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorded = applied(&runner, &dir, "gamemoderun %command%");

    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, SteamLaunchOptions::id_const(), &runner, &mut journal);

    step("gamemoderun %command%")
        .rollback(&recorded, &mut apply)
        .expect("rolled back");

    assert_eq!(runner.file(CONFIG).as_deref(), Some(config_text().as_str()));
}

#[test]
fn verify_fails_while_the_options_are_still_unset() {
    let runner = machine();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);

    let verification = step("gamemoderun %command%").verify(&cx).expect("verified");
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn verify_passes_once_the_config_says_what_was_asked_for() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    applied(&runner, &dir, "gamemoderun %command%");

    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    assert!(
        step("gamemoderun %command%")
            .verify(&cx)
            .expect("verified")
            .passed()
    );
}
