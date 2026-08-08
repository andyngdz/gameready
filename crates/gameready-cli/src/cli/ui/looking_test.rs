use gameready_core::facts::{Family, SystemFacts};

use super::{LookingAtMachine, SteamGames};

fn rendered(games: SteamGames) -> String {
    let facts = SystemFacts::fixture(Family::Arch);
    console::strip_ansi_codes(&LookingAtMachine::new(&facts, games).to_string()).into_owned()
}

#[test]
fn the_first_line_promises_nothing_has_moved_yet() {
    let screen = rendered(SteamGames::Found(3));

    assert!(screen.contains("Looking at your machine."), "{screen}");
    assert!(
        screen.contains("Nothing changes until you say so."),
        "{screen}"
    );
}

#[test]
fn the_machine_reads_as_its_own_name_kernel_and_package_manager() {
    let facts = SystemFacts::fixture(Family::Arch);
    let screen = rendered(SteamGames::Found(3));

    assert!(screen.contains(&facts.distro.name), "{screen}");
    assert!(screen.contains(&facts.kernel_release), "{screen}");
    assert!(screen.contains("pacman"), "{screen}");
}

#[test]
fn the_games_line_counts_what_a_run_could_tune() {
    assert!(
        rendered(SteamGames::Found(3)).contains("3 games I can tune"),
        "{}",
        rendered(SteamGames::Found(3))
    );
}

#[test]
fn one_game_is_not_reported_as_one_games() {
    assert!(
        rendered(SteamGames::Found(1)).contains("1 game I can tune"),
        "{}",
        rendered(SteamGames::Found(1))
    );
}

#[test]
fn steam_with_nothing_installed_is_not_the_same_line_as_no_steam() {
    // Both are zero games, and a user needs to know which of the two their
    // machine is before deciding whether the per-game questions are missing.
    let empty = rendered(SteamGames::Found(0));
    let missing = rendered(SteamGames::Missing);

    assert!(empty.contains("no games installed yet"), "{empty}");
    assert!(missing.contains("No Steam here"), "{missing}");
}
