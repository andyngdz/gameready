use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::Family;
use crate::games::AppId;
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths};
use crate::steps::CompatRank;

const LOCAL: &str = "/steam/userdata/1/config/localconfig.vdf";
const INSTALL: &str = "/steam/config/config.vdf";

fn local_text() -> String {
    indoc! {r#"
        "UserLocalConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "apps"
                        {
                            "1422450"
                            {
                                "LastPlayed"        "1"
                            }
                        }
                    }
                }
            }
        }
    "#}
    .to_owned()
}

fn install_text() -> String {
    indoc! {r#"
        "InstallConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "AutoUpdateWindowEnabled"        "0"
                    }
                }
            }
        }
    "#}
    .to_owned()
}

/// A machine with Steam already stopped, so no shutdown wait is involved.
fn stopped_steam() -> MockRunner {
    MockRunner::new()
        .failing("pgrep -x steam")
        .with_file(LOCAL, local_text())
        .with_file(INSTALL, install_text())
}

fn configs() -> SteamConfigs {
    SteamConfigs {
        local: LOCAL.into(),
        install: INSTALL.into(),
    }
}

fn launch_target() -> LaunchTarget {
    LaunchTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        options: "gamemoderun %command%".to_owned(),
    }
}

fn proton_target() -> CompatTarget {
    CompatTarget {
        app_id: AppId(1_422_450),
        name: "Deadlock".to_owned(),
        tool: "GE-Proton11-3".to_owned(),
        rank: CompatRank::Game,
    }
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("journal")
}

/// Both files go through one `write_steam_settings` call, which is what keeps
/// Steam being stopped once rather than once per file. The count of shutdowns
/// is not asserted directly: `MockRunner` answers `pgrep` the same way every
/// time, so it cannot play a Steam that was running and then stopped.
#[test]
fn both_files_are_written_once_steam_is_stopped() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    let report = write_steam_settings(
        &runner,
        &facts,
        &mut journal(&dir),
        configs(),
        SteamSettings {
            launch: vec![launch_target()],
            proton: vec![proton_target()],
        },
    )
    .expect("written");

    assert_eq!(report.steps.len(), 2, "both Steam steps should have run");
    assert!(runner
        .file(LOCAL)
        .expect("local config")
        .contains("gamemoderun %command%"));
    assert!(runner
        .file(INSTALL)
        .expect("install config")
        .contains("GE-Proton11-3"));
}

#[test]
fn a_steam_that_is_already_stopped_is_not_asked_to_quit() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    write_steam_settings(
        &runner,
        &facts,
        &mut journal(&dir),
        configs(),
        SteamSettings {
            launch: vec![launch_target()],
            proton: Vec::new(),
        },
    )
    .expect("written");

    assert!(
        !runner
            .commands()
            .iter()
            .any(|command| command.contains("-shutdown")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn nothing_to_set_changes_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    write_steam_settings(
        &runner,
        &facts,
        &mut journal(&dir),
        configs(),
        SteamSettings::default(),
    )
    .expect("nothing to do");

    assert_eq!(runner.file(LOCAL).as_deref(), Some(local_text().as_str()));
    assert_eq!(
        runner.file(INSTALL).as_deref(),
        Some(install_text().as_str())
    );
}

#[test]
fn a_run_with_only_a_proton_pin_leaves_the_launch_options_alone() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let facts = SystemFacts::fixture(Family::Debian);

    write_steam_settings(
        &runner,
        &facts,
        &mut journal(&dir),
        configs(),
        SteamSettings {
            launch: Vec::new(),
            proton: vec![proton_target()],
        },
    )
    .expect("written");

    assert_eq!(runner.file(LOCAL).as_deref(), Some(local_text().as_str()));
    assert!(runner
        .file(INSTALL)
        .expect("install config")
        .contains("GE-Proton11-3"));
}
