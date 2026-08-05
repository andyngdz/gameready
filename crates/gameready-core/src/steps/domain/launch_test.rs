use indoc::indoc;

use super::*;

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
        						"LaunchOptions"		"LD_PRELOAD="
        					}
        					"1091500"
        					{
        						"LastPlayed"		"1"
        					}
        				}
        			}
        		}
        	}
        }
    "#}
    .to_owned()
}

fn target(app_id: u32, name: &str, options: &str) -> LaunchTarget {
    LaunchTarget {
        app_id: AppId(app_id),
        name: name.to_owned(),
        options: options.to_owned(),
    }
}

#[test]
fn every_target_lands_in_one_edited_config() {
    // One pass so the file is written once. A write per game would leave the
    // config half updated if the run were interrupted between two of them.
    let edited = apply_targets(
        &config(),
        &[
            target(1_422_450, "Deadlock", "gamemoderun %command%"),
            target(1_091_500, "Cyberpunk 2077", "mangohud %command%"),
        ],
    )
    .expect("edited");

    assert_eq!(edited.replaced.len(), 2);
    assert!(edited.text.contains("gamemoderun %command%"));
    assert!(edited.text.contains("mangohud %command%"));
}

#[test]
fn the_value_a_target_displaced_is_reported() {
    // The user typed LD_PRELOAD= themselves, and overwriting it silently would
    // take away a setting they made on purpose.
    let edited = apply_targets(
        &config(),
        &[target(1_422_450, "Deadlock", "gamemoderun %command%")],
    )
    .expect("edited");

    assert_eq!(edited.replaced[0].1, "LD_PRELOAD=");
}

#[test]
fn a_target_that_is_already_set_is_not_counted_as_a_change() {
    let edited =
        apply_targets(&config(), &[target(1_422_450, "Deadlock", "LD_PRELOAD=")]).expect("edited");

    assert!(edited.replaced.is_empty());
    assert_eq!(edited.text, config());
}

#[test]
fn a_pending_game_is_reported_until_it_is_written() {
    let edited = apply_targets(
        &config(),
        &[target(1_091_500, "Cyberpunk 2077", "mangohud %command%")],
    )
    .expect("edited");

    assert!(edited.is_pending(AppId(1_091_500)));
    assert!(!edited.is_pending(AppId(1_422_450)));
}

#[test]
fn no_targets_leaves_the_config_untouched() {
    let edited = apply_targets(&config(), &[]).expect("edited");
    assert_eq!(edited.text, config());
    assert!(edited.replaced.is_empty());
}

#[test]
fn a_game_that_steam_has_never_seen_is_refused() {
    // Inventing the section would put a game Steam does not know about into a
    // file Steam owns.
    let error = apply_targets(&config(), &[target(999_999, "Nope", "mangohud %command%")])
        .expect_err("refused");
    assert!(
        matches!(error, VdfError::MissingSection { .. }),
        "{error:?}"
    );
}
