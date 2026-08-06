use super::*;

/// A non-game set holding exactly these appids, built without touching a file.
fn non_games_of(app_ids: &[u32]) -> NonGameApps {
    NonGameApps {
        ids: app_ids.iter().copied().collect(),
    }
}

#[test]
fn an_appid_steam_types_as_non_game_is_reported_so() {
    let non_games = non_games_of(&[1_495_710]);
    assert!(non_games.contains(AppId(1_495_710)));
}

#[test]
fn an_appid_not_in_the_set_is_treated_as_a_game() {
    let non_games = non_games_of(&[1_495_710]);
    assert!(!non_games.contains(AppId(1_091_500)));
}

#[test]
fn a_steam_root_without_appinfo_yields_an_empty_set() {
    // Degrade-safe: no appcache/appinfo.vdf means nothing is filtered, so a
    // missing or unreadable file never hides a real game.
    let empty = tempfile::TempDir::new().expect("temp dir");
    let non_games = NonGameApps::read(empty.path());
    assert!(!non_games.contains(AppId(1_495_710)));
}

#[test]
#[ignore = "reads this machine's real appinfo.vdf; run locally, not in CI"]
fn the_real_appinfo_separates_the_bonus_content_from_the_game() {
    // Cyberpunk 2077 Bonus Content (1495710) is typed "Music"; Cyberpunk 2077
    // (1091500) is "Game". Their appmanifests are indistinguishable, so this
    // proves the type comes from appinfo.vdf and nothing else.
    let steam = steamlocate::SteamDir::locate().expect("Steam on this machine");
    let non_games = NonGameApps::read(steam.path());
    assert!(
        non_games.contains(AppId(1_495_710)),
        "Cyberpunk 2077 Bonus Content should read as non-game"
    );
    assert!(
        !non_games.contains(AppId(1_091_500)),
        "Cyberpunk 2077 should read as a game"
    );
}
