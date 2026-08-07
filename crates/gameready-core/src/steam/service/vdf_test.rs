use indoc::indoc;

use super::*;

/// The shape of a real `localconfig.vdf`, cut down to what the edit walks.
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

fn set(text: &str, app: &str, value: &str) -> SetResult {
    let mut path = APPS.to_vec();
    path.push(app);
    set_scalar(text, &path, "LaunchOptions", value).expect("edited")
}

#[test]
fn an_existing_value_is_replaced_and_the_old_one_reported() {
    let SetResult::Changed(edit) = set(&config(), "730", "mangohud %command%") else {
        panic!("expected a change");
    };

    assert_eq!(edit.previous, "LD_PRELOAD=");
    assert!(edit.text.contains("mangohud %command%"), "{}", edit.text);
    assert!(!edit.text.contains("LD_PRELOAD="));
}

#[test]
fn a_missing_key_is_inserted_into_the_right_app() {
    let SetResult::Changed(edit) = set(&config(), "1422450", "gamemoderun %command%") else {
        panic!("expected a change");
    };

    assert_eq!(edit.previous, "");
    // The other app's value must be untouched.
    assert!(edit.text.contains("LD_PRELOAD="), "{}", edit.text);
    assert!(edit.text.contains("gamemoderun %command%"));
}

#[test]
fn setting_the_value_it_already_has_is_not_a_change() {
    // Journalling a write that changes nothing would put a rollback entry in
    // front of the user for a file gameready never actually altered.
    assert_eq!(set(&config(), "730", "LD_PRELOAD="), SetResult::AlreadySet);
}

#[test]
fn every_other_value_in_the_file_survives_the_round_trip() {
    // This file holds a user's cloud sync state and playtime for every game
    // they own. The parser reorders keys and normalises indentation, neither of
    // which Steam cares about, but losing a value would be a setting the user
    // never gets back.
    let original = config();
    let SetResult::Changed(edit) = set(&original, "730", "mangohud %command%") else {
        panic!("expected a change");
    };

    assert_eq!(
        leaves(&original.replace("LD_PRELOAD=", "@")),
        leaves(&edit.text.replace("mangohud %command%", "@"))
    );
}

#[test]
fn writing_the_same_value_twice_is_recognised_as_no_change() {
    // The parser re-renders the whole document. An unstable render would make
    // every run report a change and journal an undo for nothing.
    let SetResult::Changed(edit) = set(&config(), "730", "mangohud %command%") else {
        panic!("expected a change");
    };
    assert_eq!(
        set(&edit.text, "730", "mangohud %command%"),
        SetResult::AlreadySet
    );
}

/// Every `"key" "value"` pair in the document, sorted, ignoring layout.
fn leaves(text: &str) -> Vec<String> {
    let mut pairs: Vec<String> = text
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix('"')?;
            let (key, rest) = rest.split_once('"')?;
            let rest = rest.trim_start().strip_prefix('"')?;
            let (value, _) = rest.split_once('"')?;
            Some(format!("{key}={value}"))
        })
        .collect();
    pairs.sort();
    pairs
}

#[test]
fn an_appid_that_appears_as_a_value_elsewhere_is_not_mistaken_for_the_key() {
    // A real localconfig has appids embedded inside opaque blobs, so a lookup
    // that matched on the text rather than on the structure would land in one.
    let text = config().replace(
        r#""LastPlayed"		"1763837497""#,
        r#""Ticket"		"1422450deadbeef""#,
    );
    let SetResult::Changed(edit) = set(&text, "1422450", "gamemoderun %command%") else {
        panic!("expected a change");
    };

    assert!(edit.text.contains("1422450deadbeef"), "{}", edit.text);
}

#[test]
fn a_file_missing_the_expected_section_is_refused() {
    let unrelated = indoc! {r#"
        "Something"
        {
        }
        "#};
    let error = set_scalar(unrelated, &APPS, "LaunchOptions", "x").expect_err("refused");
    assert!(
        matches!(error, VdfError::MissingSection { .. }),
        "{error:?}"
    );
}

#[test]
fn an_app_that_is_not_in_the_file_is_refused_rather_than_guessed_at() {
    let mut path = APPS.to_vec();
    path.push("999999");
    let error = set_scalar(&config(), &path, "LaunchOptions", "x").expect_err("refused");
    assert!(
        matches!(error, VdfError::MissingSection { .. }),
        "{error:?}"
    );
}

#[test]
fn a_quote_in_the_value_is_escaped_and_read_back_unchanged() {
    let SetResult::Changed(edit) = set(&config(), "1422450", r#"FOO="bar" %command%"#) else {
        panic!("expected a change");
    };
    assert!(
        edit.text.contains(r#"FOO=\"bar\" %command%"#),
        "{}",
        edit.text
    );

    // And setting the same value again is recognised as already set, which only
    // works if the escape and the unescape agree.
    assert_eq!(
        set(&edit.text, "1422450", r#"FOO="bar" %command%"#),
        SetResult::AlreadySet
    );
}

#[test]
fn a_brace_inside_a_string_does_not_break_the_nesting() {
    let text = config().replace(r#""LD_PRELOAD=""#, r#""weird{value}here""#);
    let SetResult::Changed(edit) = set(&text, "1422450", "gamemoderun %command%") else {
        panic!("expected a change");
    };
    assert!(edit.text.contains("weird{value}here"), "{}", edit.text);
    assert!(edit.text.contains("gamemoderun %command%"));
}

#[test]
fn an_inserted_key_sits_inside_its_own_app_block() {
    // Indentation is the parser's to decide; what matters is that the key lands
    // under the game it belongs to and not a sibling.
    let SetResult::Changed(edit) = set(&config(), "1422450", "gamemoderun %command%") else {
        panic!("expected a change");
    };

    let block = edit
        .text
        .split("\"1422450\"")
        .nth(1)
        .expect("the app block");
    let block = &block[..block.find('}').expect("the block ends")];
    assert!(block.contains("gamemoderun %command%"), "{block}");
}

#[test]
fn a_truncated_file_is_refused_rather_than_written_to() {
    let truncated = indoc! {r#"
        "UserLocalConfigStore"
        {
        	"Software"
        "#};
    let error = set_scalar(truncated, &APPS, "LaunchOptions", "x").expect_err("refused");
    assert!(matches!(error, VdfError::Malformed { .. }), "{error:?}");
}

#[test]
fn a_path_that_runs_into_a_value_is_refused() {
    // "LastPlayed" holds a number, not a block, so descending through it would
    // mean writing into the wrong shape.
    let mut path = APPS.to_vec();
    path.push("730");
    path.push("LastPlayed");
    let error = set_scalar(&config(), &path, "LaunchOptions", "x").expect_err("refused");
    assert!(matches!(error, VdfError::NotABlock), "{error:?}");
}

/// The path a compatibility-tool entry lives at, one game deep.
const MAPPING: [&str; 6] = [
    "UserLocalConfigStore",
    "Software",
    "Valve",
    "Steam",
    "CompatToolMapping",
    "1422450",
];

#[test]
fn a_block_is_created_along_with_every_missing_section_above_it() {
    let values = [("name", "GE-Proton11-3"), ("priority", "250")];

    let result = set_block(&config(), &MAPPING, &values, "name").expect("set");

    let SetResult::Changed(edit) = result else {
        panic!("expected a change, got {result:?}");
    };
    assert!(edit.text.contains("CompatToolMapping"), "{}", edit.text);
    assert!(edit.text.contains("GE-Proton11-3"), "{}", edit.text);
    assert_eq!(edit.previous, "");
}

#[test]
fn the_previous_value_reported_is_the_key_the_caller_named() {
    let first = set_block(
        &config(),
        &MAPPING,
        &[("name", "GE-Proton9-20"), ("priority", "250")],
        "name",
    )
    .expect("set");
    let SetResult::Changed(first) = first else {
        panic!("expected a change");
    };

    let second = set_block(
        &first.text,
        &MAPPING,
        &[("name", "GE-Proton11-3"), ("priority", "250")],
        "name",
    )
    .expect("set");
    let SetResult::Changed(second) = second else {
        panic!("expected a change");
    };

    assert_eq!(second.previous, "GE-Proton9-20");
}

#[test]
fn a_block_that_already_says_all_of_it_is_left_alone() {
    let values = [("name", "GE-Proton11-3"), ("priority", "250")];
    let first = set_block(&config(), &MAPPING, &values, "name").expect("set");
    let SetResult::Changed(first) = first else {
        panic!("expected a change");
    };

    let again = set_block(&first.text, &MAPPING, &values, "name").expect("set");

    assert!(matches!(again, SetResult::AlreadySet), "{again:?}");
}

#[test]
fn one_key_out_of_date_rewrites_the_whole_block() {
    // Steam re-renders an entry with every key it expects, so a half-written
    // one reads as gameready's change being partly undone on the next exit.
    let first = set_block(
        &config(),
        &MAPPING,
        &[("name", "GE-Proton11-3"), ("priority", "75")],
        "name",
    )
    .expect("set");
    let SetResult::Changed(first) = first else {
        panic!("expected a change");
    };

    let second = set_block(
        &first.text,
        &MAPPING,
        &[("name", "GE-Proton11-3"), ("priority", "250")],
        "name",
    )
    .expect("set");

    assert!(matches!(second, SetResult::Changed(_)), "{second:?}");
}

#[test]
fn a_block_write_refuses_a_root_key_the_file_does_not_have() {
    let wrong = ["InstallConfigStore", "Software"];
    let error = set_block(&config(), &wrong, &[("name", "x")], "name").expect_err("refused");

    assert!(
        matches!(error, VdfError::MissingSection { .. }),
        "{error:?}"
    );
}
