use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::games::AppId;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steam::{PriorBlock, PriorScalar};

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

/// Reverses a recorded apply through the step's own rollback.
fn roll_back(runner: &MockRunner, dir: &TempDir, recorded: &[Change]) {
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, runner);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, SteamLaunchOptions::id_const(), runner, &mut journal);

    step("gamemoderun %command%")
        .rollback(recorded, &mut apply)
        .expect("rolled back");
}

#[test]
fn apply_writes_the_config_and_journals_what_the_key_held() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    let recorded = applied(&runner, &dir, "gamemoderun %command%");

    match recorded.as_slice() {
        [Change::SteamConfigWritten { sections, .. }] => {
            assert_eq!(sections.len(), 1);
            assert_eq!(
                sections[0].prior,
                PriorBlock::Present {
                    entries: vec![PriorScalar {
                        key: "LaunchOptions".to_owned(),
                        value: Some("LD_PRELOAD=".to_owned()),
                    }],
                }
            );
        }
        other => panic!("expected one Steam config write, got {other:?}"),
    }
    assert!(runner
        .file(CONFIG)
        .expect("config")
        .contains("gamemoderun %command%"));
}

#[test]
fn apply_keeps_no_copy_of_a_file_holding_credentials() {
    // localconfig.vdf carries an encrypted app ticket and a cloud key. Nothing
    // reads a pre-image any more, so keeping one in a directory nothing prunes
    // would be a copy of those for no purpose.
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();

    applied(&runner, &dir, "gamemoderun %command%");

    let copies: Vec<_> = runner
        .paths()
        .into_iter()
        .filter(|path| path != CONFIG)
        .collect();
    assert!(copies.is_empty(), "a pre-image was written: {copies:?}");
}

#[test]
fn rollback_puts_the_users_own_launch_options_back() {
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorded = applied(&runner, &dir, "gamemoderun %command%");

    roll_back(&runner, &dir, &recorded);

    let after = runner.file(CONFIG).expect("still there");
    assert!(after.contains("LD_PRELOAD="), "{after}");
    assert!(!after.contains("gamemoderun"), "{after}");
}

#[test]
fn rollback_keeps_what_steam_wrote_after_the_run() {
    // Steam saves this file every time it exits, so anything it wrote between
    // the run and the rollback has to survive. Restoring a pre-image of the
    // whole file would throw away the user's week of Steam settings.
    let dir = TempDir::new().expect("temp dir");
    let runner = machine();
    let recorded = applied(&runner, &dir, "gamemoderun %command%");

    let written = runner.file(CONFIG).expect("written");
    let steam_wrote = written.replace(
        r#""LaunchOptions"#,
        "\"LastPlayed\"\t\t\"1799999999\"\n\t\t\t\t\t\t\"LaunchOptions",
    );
    let runner = MockRunner::new().with_file(CONFIG, &steam_wrote);

    roll_back(&runner, &dir, &recorded);

    let after = runner.file(CONFIG).expect("still there");
    assert!(after.contains("1799999999"), "{after}");
    assert!(after.contains("LD_PRELOAD="), "{after}");
    assert!(!after.contains("gamemoderun"), "{after}");
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
    assert!(step("gamemoderun %command%")
        .verify(&cx)
        .expect("verified")
        .passed());
}
