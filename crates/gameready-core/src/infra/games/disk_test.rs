use indoc::indoc;
use tempfile::TempDir;

use super::*;

fn write_profile(root: &TempDir, game: &str, body: &str) {
    let dir = root.path().join(game);
    std::fs::create_dir_all(&dir).expect("create game dir");
    std::fs::write(dir.join(PROFILE_FILE), body).expect("write profile");
}

#[test]
fn a_missing_directory_is_not_an_error() {
    // Two of the three catalog layers are absent on a normal machine.
    let (profiles, failures) = profiles_in(Path::new("/nonexistent/gameready/games"));
    assert!(profiles.is_empty());
    assert!(failures.is_empty());
}

#[test]
fn every_game_directory_is_read() {
    let root = TempDir::new().expect("temp dir");
    write_profile(
        &root,
        "Deadlock",
        indoc! {r#"
            name = "Deadlock"
            steam_appid = 1422450
        "#},
    );
    write_profile(
        &root,
        "Hades",
        indoc! {r#"
            name = "Hades"
            steam_appid = 1145360
        "#},
    );

    let (mut profiles, failures) = profiles_in(root.path());
    profiles.sort_by(|a, b| a.name.cmp(&b.name));

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(profiles.len(), 2);
    assert_eq!(profiles[0].name, "Deadlock");
}

#[test]
fn a_broken_profile_costs_only_its_own_game() {
    let root = TempDir::new().expect("temp dir");
    write_profile(
        &root,
        "Good",
        indoc! {r#"
            name = "Good"
            steam_appid = 1
        "#},
    );
    write_profile(&root, "Broken", "this is not toml");

    let (profiles, failures) = profiles_in(root.path());

    assert_eq!(profiles.len(), 1);
    assert_eq!(failures.len(), 1);
}

#[test]
fn a_directory_without_a_game_toml_is_skipped() {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(root.path().join("Notes")).expect("create dir");

    let (profiles, failures) = profiles_in(root.path());
    assert!(profiles.is_empty());
    assert!(failures.is_empty());
}
