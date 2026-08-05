use super::*;

fn profile() -> GameProfile {
    GameProfile {
        name: "Cyberpunk 2077".to_owned(),
        app_id: AppId(1091500),
        wrappers: vec![Wrapper::GameMode],
        env: BTreeMap::new(),
        proton: None,
        override_module: None,
    }
}

#[test]
fn a_profile_keys_itself_off_its_display_name() {
    assert_eq!(profile().key().as_str(), "cyberpunk-2077");
}

#[test]
fn a_game_ref_carries_the_name_a_user_recognises() {
    let game = profile().game_ref();
    assert_eq!(game.name, "Cyberpunk 2077");
    assert_eq!(game.app_id, AppId(1091500));
    assert_eq!(game.key.as_str(), "cyberpunk-2077");
}

#[test]
fn gamemode_is_launched_through_its_wrapper_script() {
    // The package installs a daemon, a library, and this script. Only the
    // script belongs on a command line.
    assert_eq!(Wrapper::GameMode.command(), "gamemoderun");
}

#[test]
fn every_wrapper_names_a_command() {
    for wrapper in [Wrapper::GameMode, Wrapper::MangoHud, Wrapper::Gamescope] {
        assert!(!wrapper.command().is_empty());
    }
}

#[test]
fn the_two_names_gameready_resolves_are_recognised() {
    assert_eq!(
        ProtonChoice::parse("GE-Proton"),
        ProtonChoice::NewestGeProton
    );
    assert_eq!(
        ProtonChoice::parse("Experimental"),
        ProtonChoice::Experimental
    );
}

#[test]
fn any_other_value_is_taken_as_an_exact_tool_name() {
    // So a profile can pin a build that did not exist when this was written.
    assert_eq!(
        ProtonChoice::parse("GE-Proton9-27"),
        ProtonChoice::Pinned {
            tool: "GE-Proton9-27".to_owned()
        }
    );
}
