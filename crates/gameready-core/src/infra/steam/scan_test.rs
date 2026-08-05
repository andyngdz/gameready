use indoc::{formatdoc, indoc};
use itertools::Itertools as _;
use tempfile::TempDir;

use super::*;

/// A Steam directory holding the given apps, shaped the way a real one is.
///
/// Built from a fixture rather than the machine's own library so the filtering
/// and the ordering are covered wherever the tests run, including a CI box with
/// no Steam at all.
fn steam_with(apps: &[(u32, &str)]) -> TempDir {
    let root = TempDir::new().expect("temp dir");
    let steamapps = root.path().join("steamapps");
    std::fs::create_dir_all(steamapps.join("common")).expect("create steamapps");

    let listed = apps
        .iter()
        .map(|(app_id, _)| format!("\t\t\t\"{app_id}\"\t\t\"1\""))
        .join("\n");

    std::fs::write(
        steamapps.join("libraryfolders.vdf"),
        formatdoc! {"
            \"libraryfolders\"
            {{
            \t\"0\"
            \t{{
            \t\t\"path\"\t\t\"{path}\"
            \t\t\"apps\"
            \t\t{{
            {listed}
            \t\t}}
            \t}}
            }}
        ",
            path = root.path().display(),
        },
    )
    .expect("write libraryfolders");

    for (app_id, name) in apps {
        std::fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            formatdoc! {"
                \"AppState\"
                {{
                \t\"appid\"\t\t\"{app_id}\"
                \t\"name\"\t\t\"{name}\"
                \t\"installdir\"\t\t\"{name}\"
                \t\"StateFlags\"\t\t\"4\"
                }}
            "},
        )
        .expect("write manifest");
    }
    root
}

#[test]
fn valve_plumbing_is_left_out_of_the_list() {
    let steam = steam_with(&[
        (1_422_450, "Deadlock"),
        (1_493_710, "Proton Experimental"),
        (228_980, "Steamworks Common Redistributables"),
        (1_628_350, "Steam Linux Runtime 3.0 (sniper)"),
    ]);

    let games = scan_installed_games_in(steam.path()).expect("scanned");
    let names: Vec<&str> = games.iter().map(|game| game.name.as_str()).collect();

    assert_eq!(names, ["Deadlock"]);
}

#[test]
fn games_come_back_in_name_order() {
    // Steam hands apps back in index order, which is neither alphabetical nor
    // stable, and this list is what the user picks from.
    let steam = steam_with(&[
        (2_868_840, "Slay the Spire 2"),
        (1_091_500, "Cyberpunk 2077"),
        (1_422_450, "Deadlock"),
    ]);

    let games = scan_installed_games_in(steam.path()).expect("scanned");
    let names: Vec<&str> = games.iter().map(|game| game.name.as_str()).collect();

    assert_eq!(names, ["Cyberpunk 2077", "Deadlock", "Slay the Spire 2"]);
}

#[test]
fn every_game_carries_its_appid_and_directory() {
    let steam = steam_with(&[(1_422_450, "Deadlock")]);

    let games = scan_installed_games_in(steam.path()).expect("scanned");

    assert_eq!(games[0].app_id.0, 1_422_450);
    assert!(
        games[0].install_dir.ends_with("steamapps/common/Deadlock"),
        "{:?}",
        games[0].install_dir
    );
}

#[test]
fn a_library_with_only_plumbing_scans_to_nothing() {
    let steam = steam_with(&[(1_493_710, "Proton Experimental")]);
    assert!(
        scan_installed_games_in(steam.path())
            .expect("scanned")
            .is_empty()
    );
}

#[test]
fn a_directory_that_is_not_steam_is_reported_as_not_installed() {
    let empty = TempDir::new().expect("temp dir");
    let error = scan_installed_games_in(empty.path()).expect_err("not steam");
    assert!(matches!(error, SteamError::NotInstalled), "{error:?}");
}

#[test]
fn the_fixture_matches_the_shape_of_a_real_manifest() {
    // Pinned so a change to the fixture that stops resembling Steam's own
    // format is caught here rather than by the scan silently finding nothing.
    let expected = indoc! {r#"
        "AppState"
        {
        	"appid"		"1422450"
    "#};
    let steam = steam_with(&[(1_422_450, "Deadlock")]);
    let written = std::fs::read_to_string(steam.path().join("steamapps/appmanifest_1422450.acf"))
        .expect("read manifest");

    assert!(written.starts_with(expected), "{written}");
}
