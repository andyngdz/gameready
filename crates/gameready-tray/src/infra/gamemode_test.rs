use std::process;

use super::*;

#[test]
fn a_process_steam_never_launched_has_no_appid() {
    // This test process was not started by Steam, so nothing should be found.
    let pid = i32::try_from(process::id()).expect("a pid fits in an i32");

    assert_eq!(steam_app_id(pid), None);
}

#[test]
fn a_pid_that_is_gone_is_read_as_no_appid_rather_than_panicking() {
    // gamemode reports a pid and the game can exit before we read /proc, so
    // this is a race that happens, not a hypothetical.
    assert_eq!(steam_app_id(-1), None);
}

#[test]
fn the_variable_read_is_the_one_steam_actually_sets() {
    // Pinned because reading the wrong name fails silently: every game would
    // simply never be recognised, and the icon would just stay white.
    assert_eq!(STEAM_APP_ID, "SteamAppId");
}
