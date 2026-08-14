use super::*;

#[test]
fn both_directories_are_named_for_the_project() {
    let state = state_dir().expect("a home directory in the test environment");
    let games = user_games_dir().expect("a home directory in the test environment");

    assert!(state.ends_with(PROJECT), "{state:?}");
    assert!(
        games.ends_with(PathBuf::from(PROJECT).join(GAMES)),
        "{games:?}"
    );
}

#[test]
fn the_two_directories_are_not_the_same_place() {
    // Profiles are the user's to keep and the state directory is gameready's
    // to prune, so a run that emptied one must never reach the other.
    assert_ne!(state_dir(), user_games_dir());
}
