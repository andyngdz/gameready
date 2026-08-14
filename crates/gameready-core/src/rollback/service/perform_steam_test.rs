use indoc::indoc;

use crate::infra::exec::MockRunner;
use crate::steam::{capture_block, set_scalar, PriorScalar, SetResult};

use super::*;

const CONFIG: &str = "/steam/userdata/1/config/localconfig.vdf";
const LAUNCH: &str = "LaunchOptions";
const APPS: [&str; 5] = ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];

fn config() -> String {
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
        						"LastPlayed"		"1785943212"
        					}
        				}
        			}
        		}
        	}
        }
    "#}
    .to_owned()
}

fn section(app: &str) -> Vec<String> {
    let mut path: Vec<String> = APPS.iter().map(|part| (*part).to_owned()).collect();
    path.push(app.to_owned());
    path
}

/// Captures the prior state, writes launch options over it, and seeds a runner
/// holding the written file.
fn applied(app: &str) -> (MockRunner, Vec<PriorSection>) {
    let text = config();
    let section = section(app);
    let borrowed: Vec<&str> = section.iter().map(String::as_str).collect();
    let prior = capture_block(&text, &borrowed, &[LAUNCH]).expect("captured");

    let SetResult::Changed(edit) =
        set_scalar(&text, &borrowed, LAUNCH, "gamemoderun %command%").expect("edited")
    else {
        panic!("expected a change");
    };

    (
        MockRunner::new().with_file(CONFIG, &edit.text),
        vec![PriorSection { section, prior }],
    )
}

#[test]
fn the_key_the_run_added_is_taken_back_out() {
    let (runner, sections) = applied("1422450");

    let outcome = restore_steam_config(&runner, Path::new(CONFIG), &sections);

    assert!(
        matches!(outcome, UndoOutcome::Reverted { .. }),
        "{outcome:?}"
    );
    let after = runner.file(CONFIG).expect("still there");
    assert!(!after.contains("gamemoderun"), "{after}");
    assert!(!after.contains(LAUNCH), "{after}");
}

#[test]
fn what_steam_wrote_after_the_run_survives_the_undo() {
    // The whole reason this undo is surgical. Steam saves the file on exit, and
    // a pre-image restore would throw that write away.
    let (runner, sections) = applied("1422450");
    let written = runner.file(CONFIG).expect("written");
    let SetResult::Changed(steam) = set_scalar(
        &written,
        &[
            "UserLocalConfigStore",
            "Software",
            "Valve",
            "Steam",
            "apps",
            "1422450",
        ],
        "LastPlayed",
        "1799999999",
    )
    .expect("edited") else {
        panic!("expected a change");
    };
    let runner = MockRunner::new().with_file(CONFIG, &steam.text);

    let outcome = restore_steam_config(&runner, Path::new(CONFIG), &sections);

    assert!(
        matches!(outcome, UndoOutcome::Reverted { .. }),
        "{outcome:?}"
    );
    let after = runner.file(CONFIG).expect("still there");
    assert!(after.contains("1799999999"), "{after}");
    assert!(!after.contains("gamemoderun"), "{after}");
}

#[test]
fn a_config_that_is_gone_needs_no_undo() {
    let (_, sections) = applied("1422450");
    let runner = MockRunner::new();

    let outcome = restore_steam_config(&runner, Path::new(CONFIG), &sections);

    assert!(matches!(outcome, UndoOutcome::AlreadyGone));
}

#[test]
fn undoing_twice_reports_nothing_left_rather_than_failing() {
    // Rollback has to be safe to re-run after a partial undo.
    let (runner, sections) = applied("1422450");
    let first = restore_steam_config(&runner, Path::new(CONFIG), &sections);
    assert!(matches!(first, UndoOutcome::Reverted { .. }));

    let second = restore_steam_config(&runner, Path::new(CONFIG), &sections);

    assert!(matches!(second, UndoOutcome::AlreadyGone), "{second:?}");
}

#[test]
fn a_config_that_stopped_being_a_config_fails_rather_than_reporting_a_revert() {
    let (_, sections) = applied("1422450");
    let runner = MockRunner::new().with_file(CONFIG, "this is not a vdf file");

    let outcome = restore_steam_config(&runner, Path::new(CONFIG), &sections);

    assert!(matches!(outcome, UndoOutcome::Failed { .. }), "{outcome:?}");
}

#[test]
fn the_row_names_how_many_settings_went_back() {
    let sections = vec![PriorSection {
        section: section("730"),
        prior: PriorBlock::Present {
            entries: vec![PriorScalar {
                key: LAUNCH.to_owned(),
                value: Some("LD_PRELOAD=".to_owned()),
            }],
        },
    }];

    assert_eq!(describe(&sections), "1 setting(s)");
}

#[test]
fn the_row_says_when_the_entry_itself_was_ours() {
    let sections = vec![PriorSection {
        section: section("730"),
        prior: PriorBlock::Absent,
    }];

    assert_eq!(describe(&sections), "1 entry(s) gameready added");
}
