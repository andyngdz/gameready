use super::*;

#[test]
fn the_games_directory_sits_under_the_config_directory() {
    // Empty only when the home directory cannot be resolved, which is not the
    // case in the test environment. When resolved it must end in the games
    // folder the CLI also reads, or the tray and the CLI see different profiles.
    let games = user_games_dir();
    assert!(
        !games.as_os_str().is_empty(),
        "a home directory should resolve"
    );
    assert_eq!(
        games.file_name().and_then(|name| name.to_str()),
        Some(GAMES)
    );
}

#[test]
fn the_state_directory_carries_the_project_name() {
    // The tray watches this directory for the journal, so it must be the same
    // path the CLI writes to: the project name is what ties the two together.
    let state = state_dir();
    assert!(
        !state.as_os_str().is_empty(),
        "a home directory should resolve"
    );
    assert!(
        state.components().any(|part| part.as_os_str() == PROJECT),
        "the state directory should carry the project name, got {state:?}",
    );
}
