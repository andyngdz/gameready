use indoc::indoc;
use tempfile::TempDir;

use super::*;

#[test]
fn the_shipped_profiles_load_without_any_directory_on_disk() {
    // The release artifact is one static binary on a machine with no gameready
    // package and no data directory.
    let empty = TempDir::new().expect("temp dir");
    let (catalog, failures) = load_catalog(empty.path());

    assert!(failures.is_empty(), "{failures:?}");
    assert!(catalog.find("Deadlock").is_some());
}

#[test]
fn a_user_profile_wins_over_the_shipped_one() {
    let user = TempDir::new().expect("temp dir");
    let dir = user.path().join("Deadlock");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(
        dir.join("game.toml"),
        indoc! {r#"
            name = "Deadlock"
            steam_appid = 1422450
        "#},
    )
    .expect("write");

    let (catalog, _) = load_catalog(user.path());
    let entry = catalog.find("Deadlock").expect("found");

    assert_eq!(entry.source, Source::User);
    assert!(
        entry.profile.wrappers.is_empty(),
        "the user's file has no [launch] table, so nothing should carry over"
    );
}

#[test]
fn a_broken_user_profile_is_reported_without_emptying_the_catalog() {
    let user = TempDir::new().expect("temp dir");
    let dir = user.path().join("Broken");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(dir.join("game.toml"), "nope").expect("write");

    let (catalog, failures) = load_catalog(user.path());

    assert_eq!(failures.len(), 1);
    assert!(catalog.find("Deadlock").is_some());
}
