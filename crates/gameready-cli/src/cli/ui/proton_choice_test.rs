use gameready_core::games::{AppId, ProtonChoice};
use gameready_core::steps::CompatRank;

use super::*;

fn game_wish() -> CompatWish {
    CompatWish {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        choice: ProtonChoice::NewestGeProton,
        rank: CompatRank::Game,
    }
}

#[test]
fn keeping_the_current_settings_writes_nothing_at_all() {
    // Not even the games that asked for a build. The answer is about the whole
    // feature, so a "no" that still rewrote two entries would be a lie.
    let wishes = ProtonPin::KeepCurrent.wishes(vec![game_wish()]);

    assert!(wishes.is_empty());
}

#[test]
fn using_the_newest_build_covers_the_games_and_everything_else() {
    let wishes = ProtonPin::UseNewest.wishes(vec![game_wish()]);

    assert_eq!(wishes.len(), 2);
    assert_eq!(wishes[0].rank, CompatRank::Game);
    assert_eq!(wishes[1].rank, CompatRank::MachineWide);
}

#[test]
fn the_machine_wide_wish_asks_for_the_newest_rather_than_a_fixed_build() {
    // An exact build here would go stale the next time one is installed, and
    // the user would be back to a default pointing at the old version.
    let machine_wide = CompatWish::machine_wide();

    assert_eq!(machine_wide.choice, ProtonChoice::NewestGeProton);
    assert_eq!(machine_wide.app_id, AppId(0));
}
