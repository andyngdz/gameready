use std::collections::BTreeMap;
use std::path::PathBuf;

use super::*;
use crate::games::{AppId, GameProfile, ProtonChoice, Wrapper};
use crate::steam::InstalledGame;

fn setup(name: &str, app_id: u32, proton: Option<ProtonChoice>) -> GameSetup {
    GameSetup {
        game: InstalledGame::new(AppId(app_id), name.to_owned(), PathBuf::from(name)),
        profile: GameProfile {
            name: name.to_owned(),
            app_id: AppId(app_id),
            wrappers: vec![Wrapper::GameMode],
            env: BTreeMap::new(),
            proton,
            override_module: None,
        },
        source: None,
    }
}

fn installed(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_profile_asking_for_ge_proton_resolves_to_the_newest_installed_build() {
    let tools = installed(&["GE-Proton9-20", "GE-Proton11-3"]);

    let targets = compat_targets_for(
        &[setup(
            "Deadlock",
            1_422_450,
            Some(ProtonChoice::NewestGeProton),
        )],
        &tools,
    );

    assert_eq!(targets.len(), 1);
    let deadlock = &targets[0];
    assert_eq!(deadlock.tool, "GE-Proton11-3");
    assert_eq!(deadlock.name, "Deadlock");
    assert_eq!(deadlock.app_id, AppId(1_422_450));
}

#[test]
fn a_game_is_left_alone_when_the_build_it_asks_for_is_not_installed() {
    // Pinning it to a build that is not there stops it launching, which is
    // worse than the version Steam would have picked.
    let targets = compat_targets_for(
        &[setup(
            "Deadlock",
            1_422_450,
            Some(ProtonChoice::NewestGeProton),
        )],
        &[],
    );

    assert!(targets.is_empty());
}

#[test]
fn experimental_resolves_without_anything_installed_by_hand() {
    // Steam ships it as an ordinary app, so it is never in compatibilitytools.d.
    let targets = compat_targets_for(
        &[setup(
            "Cyberpunk 2077",
            1_091_500,
            Some(ProtonChoice::Experimental),
        )],
        &[],
    );

    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].tool, PROTON_EXPERIMENTAL);
}

#[test]
fn an_exact_tool_name_is_used_as_written() {
    let targets = compat_targets_for(
        &[setup(
            "Some Game",
            1,
            Some(ProtonChoice::Pinned {
                tool: "GE-Proton8-32".to_owned(),
            }),
        )],
        &installed(&["GE-Proton11-3"]),
    );

    assert_eq!(targets[0].tool, "GE-Proton8-32");
}

#[test]
fn a_profile_that_says_nothing_about_proton_pins_nothing() {
    // Steam's own choice is the default, and overwriting it for a game nobody
    // said anything about is a change nobody asked for.
    let targets = compat_targets_for(
        &[setup("Hollow Knight", 367_520, None)],
        &installed(&["GE-Proton11-3"]),
    );

    assert!(targets.is_empty());
}
