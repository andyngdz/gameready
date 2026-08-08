use indoc::indoc;
use tempfile::TempDir;

use super::run;

#[test]
fn the_shipped_profiles_are_listed_with_no_user_directory() {
    let empty = TempDir::new().expect("temp dir");
    let text = run(empty.path()).expect("catalog loads");

    assert!(text.contains("Deadlock"), "{text}");
    assert!(text.contains("built in"), "{text}");
}

#[test]
fn a_user_profile_is_listed_as_theirs() {
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

    let text = run(user.path()).expect("catalog loads");
    assert!(text.contains("yours"), "{text}");
}

#[test]
fn a_broken_user_profile_is_named_without_hiding_the_rest() {
    let user = TempDir::new().expect("temp dir");
    let dir = user.path().join("Broken");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(dir.join("game.toml"), "nope").expect("write");

    let text = run(user.path()).expect("catalog loads");
    assert!(text.contains("Couldn't read 1 file"), "{text}");
    assert!(text.contains("Deadlock"), "{text}");
}
