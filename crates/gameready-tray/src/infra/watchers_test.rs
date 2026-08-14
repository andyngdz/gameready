use super::*;

#[test]
fn the_tray_reads_the_directories_the_cli_writes() {
    // Both sides now resolve through gameready_core::infra::dirs, so a rename
    // cannot move the CLI's paths without moving the tray's. What is left to
    // check here is that the tray still delegates rather than deriving its own.
    // Empty only when the home directory cannot be resolved, which is not the
    // case in the test environment.
    let games = dirs::user_games_dir().expect("a home directory in the test environment");
    let state = dirs::state_dir().expect("a home directory in the test environment");

    assert_eq!(user_games_dir(), games);
    assert_eq!(state_dir(), state);
}
