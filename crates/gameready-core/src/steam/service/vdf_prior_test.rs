use indoc::indoc;

use super::*;
use crate::steam::{set_block, set_scalar, SetResult};

/// The shape of a real `localconfig.vdf`, cut down to what the edit walks.
///
/// App 730 has launch options already; 1422450 has none, which is the two cases
/// an undo has to tell apart.
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
                            "730"
                            {
                                "LastPlayed"		"1763837497"
                                "LaunchOptions"		"LD_PRELOAD="
                            }
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

const APPS: [&str; 5] = ["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"];
const LAUNCH: &str = "LaunchOptions";

fn app_path(app: &str) -> Vec<&str> {
    let mut path = APPS.to_vec();
    path.push(app);
    path
}

/// Captures the prior state, writes over it, and hands back both.
fn apply_launch_options(app: &str, value: &str) -> (PriorBlock, String) {
    let text = config();
    let path = app_path(app);
    let prior = capture_block(&text, &path, &[LAUNCH]).expect("captured");

    let SetResult::Changed(edit) = set_scalar(&text, &path, LAUNCH, value).expect("edited") else {
        panic!("expected a change");
    };
    (prior, edit.text)
}

#[test]
fn a_key_that_was_there_is_captured_with_its_value() {
    let prior = capture_block(&config(), &app_path("730"), &[LAUNCH]).expect("captured");

    assert_eq!(
        prior,
        PriorBlock::Present {
            entries: vec![PriorScalar {
                key: LAUNCH.to_owned(),
                value: Some("LD_PRELOAD=".to_owned()),
            }],
        }
    );
}

#[test]
fn a_key_that_was_absent_is_captured_as_absent_rather_than_empty() {
    // An empty string is a real launch options setting. Recording one for a key
    // that was never there would leave it behind on undo.
    let prior = capture_block(&config(), &app_path("1422450"), &[LAUNCH]).expect("captured");

    assert_eq!(
        prior,
        PriorBlock::Present {
            entries: vec![PriorScalar {
                key: LAUNCH.to_owned(),
                value: None,
            }],
        }
    );
}

#[test]
fn a_block_the_file_does_not_have_is_captured_as_absent() {
    let prior = capture_block(&config(), &app_path("999999"), &[LAUNCH]).expect("captured");

    assert_eq!(prior, PriorBlock::Absent);
}

#[test]
fn undoing_puts_the_previous_value_back() {
    let (prior, written) = apply_launch_options("730", "gamemoderun %command%");

    let restored = restore_block(&written, &app_path("730"), &prior).expect("restored");

    assert!(restored.contains("LD_PRELOAD="), "{restored}");
    assert!(!restored.contains("gamemoderun"), "{restored}");
}

#[test]
fn undoing_removes_a_key_the_run_added() {
    let (prior, written) = apply_launch_options("1422450", "gamemoderun %command%");
    assert_eq!(written.matches(LAUNCH).count(), 2, "{written}");

    let restored = restore_block(&written, &app_path("1422450"), &prior).expect("restored");

    // Down to the one app 730 had all along, so the added key went and the
    // untouched one stayed.
    assert_eq!(restored.matches(LAUNCH).count(), 1, "{restored}");
    assert!(!restored.contains("gamemoderun"), "{restored}");
    // The rest of the block is Steam's, not ours to remove with it.
    assert!(restored.contains("1785943212"), "{restored}");
}

#[test]
fn undoing_leaves_what_steam_wrote_after_the_run() {
    // The whole reason this exists instead of restoring a pre-image: Steam
    // saves the file on exit, and that write must survive the undo.
    let (prior, written) = apply_launch_options("730", "gamemoderun %command%");
    let SetResult::Changed(steam_wrote) =
        set_scalar(&written, &app_path("730"), "LastPlayed", "1799999999").expect("edited")
    else {
        panic!("expected a change");
    };

    let restored = restore_block(&steam_wrote.text, &app_path("730"), &prior).expect("restored");

    assert!(restored.contains("1799999999"), "{restored}");
    assert!(restored.contains("LD_PRELOAD="), "{restored}");
}

#[test]
fn undoing_an_absent_block_takes_the_whole_entry_back_out() {
    let text = config();
    let path = app_path("999999");
    let prior = capture_block(&text, &path, &[LAUNCH]).expect("captured");
    let SetResult::Changed(edit) =
        set_block(&text, &path, &[(LAUNCH, "gamemoderun %command%")], LAUNCH).expect("edited")
    else {
        panic!("expected a change");
    };
    assert!(edit.text.contains("999999"), "{}", edit.text);

    let restored = restore_block(&edit.text, &path, &prior).expect("restored");

    assert!(!restored.contains("999999"), "{restored}");
    assert!(restored.contains("1422450"), "{restored}");
}

#[test]
fn undoing_twice_is_safe() {
    // Rollback can be re-run after a partial undo, so the second pass must not
    // fail on a block the first one already took out.
    let text = config();
    let path = app_path("999999");
    let prior = capture_block(&text, &path, &[LAUNCH]).expect("captured");

    let once = restore_block(&text, &path, &prior).expect("first");
    let twice = restore_block(&once, &path, &prior).expect("second");

    assert_eq!(once, twice);
}

#[test]
fn undoing_a_block_steam_dropped_invents_nothing() {
    let text = config();
    let prior = capture_block(&text, &app_path("730"), &[LAUNCH]).expect("captured");

    // Steam removed the app entry entirely, the user having uninstalled it.
    let without = restore_block(&text, &app_path("730"), &PriorBlock::Absent).expect("removed");
    let restored = restore_block(&without, &app_path("730"), &prior).expect("restored");

    assert!(!restored.contains("730"), "{restored}");
}

#[test]
fn a_file_whose_root_is_not_the_expected_one_is_refused() {
    let path = ["SomethingElse", "apps", "730"];

    assert!(capture_block(&config(), &path, &[LAUNCH]).is_err());
    assert!(restore_block(&config(), &path, &PriorBlock::Absent).is_err());
}
