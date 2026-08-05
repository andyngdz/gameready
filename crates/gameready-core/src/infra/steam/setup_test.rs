use tempfile::TempDir;

use super::*;

#[test]
fn a_machine_without_steam_still_yields_a_list_rather_than_an_error() {
    // The core tuning does not need Steam, so the run must survive its absence.
    // On a machine that does have Steam this simply returns what is installed.
    let empty = TempDir::new().expect("temp dir");
    let setups = discover_setups(empty.path());

    for setup in &setups {
        assert!(!setup.game.name.is_empty());
    }
}
