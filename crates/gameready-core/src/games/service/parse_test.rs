use std::path::Path;

use indoc::indoc;

use super::*;
use crate::games::domain::AppId;

fn parse(text: &str) -> Result<GameProfile, GameError> {
    parse_profile(Path::new("games/Test/game.toml"), text)
}

#[test]
fn a_minimal_profile_needs_only_a_name_and_an_appid() {
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450
    "#})
    .expect("parsed");

    assert_eq!(profile.name, "Deadlock");
    assert_eq!(profile.app_id, AppId(1422450));
    assert_eq!(profile.wrappers, default_wrappers());
    assert!(profile.env.is_empty());
    assert_eq!(profile.proton, None);
}

#[test]
fn wrappers_nest_gamemode_outermost_whatever_order_the_file_lists() {
    // gamemode has to cover everything below it, and gamescope has to sit
    // outside the game but inside gamemode. A file that could reorder these
    // would mostly reorder them wrong.
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [launch]
        mangohud = true
        gamescope = true
        gamemode = true
    "#})
    .expect("parsed");

    assert_eq!(
        profile.wrappers,
        vec![Wrapper::GameMode, Wrapper::Gamescope, Wrapper::MangoHud]
    );
}

#[test]
fn a_launch_flag_left_out_means_off() {
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [launch]
        gamemode = true
    "#})
    .expect("parsed");

    assert_eq!(profile.wrappers, vec![Wrapper::GameMode]);
}

#[test]
fn a_profile_that_says_nothing_about_gamemode_still_gets_it() {
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450
    "#})
    .expect("parsed");

    assert_eq!(profile.wrappers, default_wrappers());
    assert_eq!(profile.wrappers, vec![Wrapper::GameMode]);
}

#[test]
fn a_profile_can_turn_gamemode_off() {
    // The only way out for a game that gamemode breaks, which is why the field
    // is three-state rather than a plain bool.
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [launch]
        gamemode = false
    "#})
    .expect("parsed");

    assert!(profile.wrappers.is_empty());
}

#[test]
fn turning_gamemode_off_leaves_the_other_wrappers_alone() {
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [launch]
        gamemode = false
        mangohud = true
    "#})
    .expect("parsed");

    assert_eq!(profile.wrappers, vec![Wrapper::MangoHud]);
}

#[test]
fn env_and_proton_come_through() {
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [env]
        DXVK_HUD = "fps"

        [proton]
        prefer = "GE-Proton"
    "#})
    .expect("parsed");

    assert_eq!(profile.env.get("DXVK_HUD").map(String::as_str), Some("fps"));
    assert_eq!(profile.proton, Some(ProtonChoice::NewestGeProton));
}

#[test]
fn an_override_module_comes_through() {
    let profile = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [override]
        module = "deadlock"
    "#})
    .expect("parsed");

    assert_eq!(profile.override_module.as_deref(), Some("deadlock"));
}

#[test]
fn a_misspelled_key_is_rejected_rather_than_ignored() {
    // Accepting it in silence would leave the user wondering why the setting
    // they wrote did nothing.
    let error = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 1422450

        [launch]
        gamemodes = true
    "#})
    .expect_err("rejected");

    assert!(matches!(error, GameError::Invalid { .. }), "{error:?}");
}

#[test]
fn a_misspelled_top_level_key_is_rejected() {
    let error = parse(indoc! {r#"
        name = "Deadlock"
        steam_app_id = 1422450
    "#})
    .expect_err("rejected");

    assert!(matches!(error, GameError::Invalid { .. }), "{error:?}");
}

#[test]
fn a_profile_with_no_usable_name_is_rejected() {
    let error = parse(indoc! {r#"
        name = "   "
        steam_appid = 1422450
    "#})
    .expect_err("rejected");

    assert!(matches!(error, GameError::NoName { .. }), "{error:?}");
}

#[test]
fn appid_zero_is_rejected() {
    let error = parse(indoc! {r#"
        name = "Deadlock"
        steam_appid = 0
    "#})
    .expect_err("rejected");

    assert!(matches!(error, GameError::NoAppId { .. }), "{error:?}");
}

#[test]
fn the_error_names_the_file_it_came_from() {
    // The catalog reads three directories, so "a game.toml is broken" without
    // a path is unactionable.
    let error = parse("this is not toml").expect_err("rejected");
    assert!(
        error.to_string().contains("games/Test/game.toml"),
        "{error}"
    );
}

#[test]
fn a_name_is_trimmed_before_it_becomes_a_key() {
    let profile = parse(indoc! {r#"
        name = "  Deadlock  "
        steam_appid = 1422450
    "#})
    .expect("parsed");

    assert_eq!(profile.name, "Deadlock");
    assert_eq!(profile.key().as_str(), "deadlock");
}
