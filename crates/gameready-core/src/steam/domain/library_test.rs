use super::*;

#[test]
fn a_game_carries_the_name_steam_shows() {
    let game = InstalledGame::new(
        AppId(1_422_450),
        "Deadlock".to_owned(),
        PathBuf::from("/games/Deadlock"),
    );

    assert_eq!(game.name, "Deadlock");
    assert_eq!(game.app_id, AppId(1_422_450));
    assert_eq!(game.install_dir, PathBuf::from("/games/Deadlock"));
}
