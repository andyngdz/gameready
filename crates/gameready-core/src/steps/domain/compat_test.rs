use indoc::indoc;

use super::*;

/// A config.vdf with the mapping Steam writes for its machine-wide default.
///
/// Shaped after this machine's own file: appid `0` at priority 75 is the
/// "run everything through this" setting, which a per-game entry has to beat.
fn config_with_default() -> String {
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
                                "name"		"GE-Proton11-3"
                                "config"		""
                                "priority"		"75"
                            }
                        }
                    }
                }
            }
        }
    "#}
    .to_owned()
}

/// A config.vdf that has never had a compatibility tool picked in it.
fn config_without_mapping() -> String {
    indoc! {r#"
        "InstallConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "AutoUpdateWindowEnabled"		"0"
                    }
                }
            }
        }
    "#}
    .to_owned()
}

fn target(app_id: u32, name: &str, tool: &str) -> CompatTarget {
    CompatTarget {
        app_id: AppId(app_id),
        name: name.to_owned(),
        tool: tool.to_owned(),
        rank: CompatRank::Game,
    }
}

fn wish(app_id: u32, name: &str, choice: ProtonChoice) -> CompatWish {
    CompatWish {
        app_id: AppId(app_id),
        name: name.to_owned(),
        choice,
        rank: CompatRank::Game,
    }
}

fn installed(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_game_gains_an_entry_next_to_the_machine_wide_default() {
    let edited = apply_compat_targets(
        &config_with_default(),
        &[target(1_422_450, "Deadlock", "GE-Proton11-3")],
    )
    .expect("edited");

    assert_eq!(edited.replaced.len(), 1);
    assert!(edited.text.contains("1422450"), "{}", edited.text);
    // The default entry is still there: a pin is an addition, not a takeover.
    assert!(edited.text.contains("\"0\""), "{}", edited.text);
}

#[test]
fn the_entry_carries_a_priority_that_beats_the_machine_wide_default() {
    // Written below 75 it would be stored and then ignored, which looks like
    // gameready doing nothing at all.
    let edited = apply_compat_targets(
        &config_with_default(),
        &[target(1_422_450, "Deadlock", "GE-Proton11-3")],
    )
    .expect("edited");

    assert!(edited.text.contains("\"250\""), "{}", edited.text);
}

#[test]
fn a_config_that_never_had_a_tool_picked_gains_the_mapping_block() {
    let edited = apply_compat_targets(
        &config_without_mapping(),
        &[target(1_422_450, "Deadlock", "GE-Proton11-3")],
    )
    .expect("edited");

    assert!(edited.text.contains("CompatToolMapping"), "{}", edited.text);
    assert!(edited.text.contains("GE-Proton11-3"), "{}", edited.text);
    // The rest of the block survives being descended through.
    assert!(
        edited.text.contains("AutoUpdateWindowEnabled"),
        "{}",
        edited.text
    );
}

#[test]
fn a_game_already_pinned_to_the_same_build_changes_nothing() {
    let once = apply_compat_targets(
        &config_with_default(),
        &[target(1_422_450, "Deadlock", "GE-Proton11-3")],
    )
    .expect("edited");

    let twice = apply_compat_targets(
        &once.text,
        &[target(1_422_450, "Deadlock", "GE-Proton11-3")],
    )
    .expect("edited");

    assert!(twice.replaced.is_empty());
    assert_eq!(twice.text, once.text);
}

#[test]
fn changing_a_pin_reports_the_build_it_displaced() {
    let first = apply_compat_targets(
        &config_with_default(),
        &[target(1_422_450, "Deadlock", "GE-Proton9-20")],
    )
    .expect("edited");

    let second = apply_compat_targets(
        &first.text,
        &[target(1_422_450, "Deadlock", "GE-Proton11-3")],
    )
    .expect("edited");

    assert_eq!(second.replaced[0].1, "GE-Proton9-20");
}

#[test]
fn every_game_lands_in_one_pass_over_the_file() {
    let edited = apply_compat_targets(
        &config_with_default(),
        &[
            target(1_422_450, "Deadlock", "GE-Proton11-3"),
            target(1_091_500, "Cyberpunk 2077", "proton_experimental"),
        ],
    )
    .expect("edited");

    assert_eq!(edited.replaced.len(), 2);
    assert!(edited.text.contains("1422450"), "{}", edited.text);
    assert!(edited.text.contains("1091500"), "{}", edited.text);
    assert!(edited.is_pending(AppId(1_091_500)));
}

#[test]
fn the_machine_wide_entry_keeps_the_rank_steam_files_it_under() {
    // Written at a game's rank it would claim every game outright, and which
    // entry Steam honoured would come down to the order it read them in.
    let targets = resolve_wishes(
        &[CompatWish::machine_wide()],
        &installed(&["GE-Proton11-5-x86_64"]),
    );

    let machine_wide_target = &targets[0];
    assert_eq!(machine_wide_target.rank, CompatRank::MachineWide);
    assert_eq!(machine_wide_target.rank.priority(), "75");
    assert_eq!(CompatRank::Game.priority(), "250");
}

#[test]
fn a_wish_for_the_newest_build_resolves_against_what_is_installed_now() {
    let wishes = [wish(1_422_450, "Deadlock", ProtonChoice::NewestGeProton)];

    let before = resolve_wishes(&wishes, &installed(&["GE-Proton11-3"]));
    let after = resolve_wishes(
        &wishes,
        &installed(&["GE-Proton11-3", "GE-Proton11-5-x86_64"]),
    );

    // The same wish, two answers. This is why resolving happens after whatever
    // installs a build, not while the run is still asking questions.
    assert_eq!(before[0].tool, "GE-Proton11-3");
    assert_eq!(after[0].tool, "GE-Proton11-5-x86_64");
}

#[test]
fn a_wish_whose_build_is_not_there_is_dropped_rather_than_written() {
    // Pinning to an absent build stops the game launching at all, which is
    // worse than the version Steam would have picked for itself.
    let targets = resolve_wishes(&[CompatWish::machine_wide()], &[]);

    assert!(targets.is_empty());
}

#[test]
fn an_exact_tool_name_resolves_without_it_being_installed() {
    // A profile naming a build gameready has never heard of is used as written:
    // the name is the user's, and second-guessing it helps nobody.
    let targets = resolve_wishes(
        &[wish(
            1,
            "Some Game",
            ProtonChoice::Pinned {
                tool: "GE-Proton8-32".to_owned(),
            },
        )],
        &installed(&["GE-Proton11-3"]),
    );

    assert_eq!(targets[0].tool, "GE-Proton8-32");
}
