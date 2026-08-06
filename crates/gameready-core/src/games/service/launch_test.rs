use std::collections::BTreeMap;

use super::*;
use crate::games::domain::AppId;

fn profile(wrappers: Vec<Wrapper>, env: &[(&str, &str)]) -> GameProfile {
    GameProfile {
        name: "Deadlock".to_owned(),
        app_id: AppId(1422450),
        wrappers,
        env: env
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect::<BTreeMap<_, _>>(),
        proton: None,
        override_module: None,
    }
}

#[test]
fn a_profile_that_asks_for_nothing_renders_nothing() {
    // Read by the caller as "leave the box alone", not "clear it".
    assert_eq!(launch_options(&profile(Vec::new(), &[])), "");
}

#[test]
fn wrappers_wrap_the_command_token() {
    assert_eq!(
        launch_options(&profile(vec![Wrapper::GameMode, Wrapper::MangoHud], &[])),
        "gamemoderun mangohud %command%"
    );
}

#[test]
fn environment_assignments_come_before_the_wrappers() {
    // A shell applies them to everything that follows, so putting them after a
    // wrapper would set them for nothing.
    assert_eq!(
        launch_options(&profile(vec![Wrapper::GameMode], &[("DXVK_HUD", "fps")])),
        "DXVK_HUD=fps gamemoderun %command%"
    );
}

#[test]
fn gamescope_gets_a_separator_so_it_does_not_eat_the_command() {
    // Without it gamescope reads everything after itself as its own flags and
    // the game never starts.
    assert_eq!(
        launch_options(&profile(
            vec![Wrapper::GameMode, Wrapper::Gamescope, Wrapper::MangoHud],
            &[]
        )),
        "gamemoderun gamescope -- mangohud %command%"
    );
}

#[test]
fn environment_alone_still_carries_the_command_token() {
    // Otherwise Steam would run the assignment and never the game.
    assert_eq!(
        launch_options(&profile(Vec::new(), &[("DXVK_HUD", "fps")])),
        "DXVK_HUD=fps %command%"
    );
}

#[test]
fn environment_order_is_the_same_on_every_machine() {
    let rendered = launch_options(&profile(
        Vec::new(),
        &[("ZZZ", "3"), ("AAA", "1"), ("MMM", "2")],
    ));
    assert_eq!(rendered, "AAA=1 MMM=2 ZZZ=3 %command%");
}

#[test]
fn the_command_token_is_always_last() {
    let rendered = launch_options(&profile(
        vec![Wrapper::GameMode, Wrapper::Gamescope],
        &[("DXVK_HUD", "fps")],
    ));
    assert!(rendered.ends_with("%command%"), "{rendered}");
}
