use super::*;

fn tool(app_id: u32, name: &str) -> bool {
    is_valve_tool(AppId(app_id), name)
}

#[test]
fn the_runtimes_on_this_machine_are_filtered() {
    // Read off a real library on 2026-08-05.
    assert!(tool(228_980, "Steamworks Common Redistributables"));
    assert!(tool(1_628_350, "Steam Linux Runtime 3.0 (sniper)"));
    assert!(tool(4_183_110, "Steam Linux Runtime 4.0"));
    assert!(tool(1_493_710, "Proton Experimental"));
}

#[test]
fn a_proton_release_published_later_is_filtered_by_name() {
    // Valve mints a new appid per release, so an appid list alone goes stale.
    assert!(tool(9_999_999, "Proton 11.0"));
}

#[test]
fn a_known_appid_is_filtered_even_under_an_unexpected_name() {
    assert!(tool(1_070_560, "something Valve renamed"));
}

#[test]
fn real_games_are_left_in() {
    assert!(!tool(1_422_450, "Deadlock"));
    assert!(!tool(1_091_500, "Cyberpunk 2077"));
    assert!(!tool(2_868_840, "Slay the Spire 2"));
}

#[test]
fn an_entry_the_filter_is_unsure_about_is_left_in() {
    // An extra row is noise the user scrolls past; a missing game is a bug they
    // cannot work around.
    assert!(!tool(1_495_710, "Cyberpunk 2077 Bonus Content"));
}

#[test]
fn a_name_that_merely_mentions_a_tool_is_not_filtered() {
    assert!(!tool(123, "The Protonaut"));
}
