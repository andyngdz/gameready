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
    }
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
