use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::improvement::ImprovementId;
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths, Undo};
use crate::rollback::{PlannedUndo, UndoOutcome};
use crate::steam::{PriorBlock, PriorScalar, PriorSection};
use crate::steps::{SteamLaunchOptions, SteamProton};

const CONFIG: &str = "/steam/config/config.vdf";

/// The config as gameready left it: the game pinned to a build it chose.
fn written_config() -> String {
    indoc! {r#"
        "InstallConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "CompatToolMapping"
                        {
                            "1422450"
                            {
                                "name"        "GE-Proton11-3"
                            }
                        }
                    }
                }
            }
        }
    "#}
    .to_owned()
}

/// Every test seeds Steam as stopped. `MockRunner` answers `pgrep` the same way
/// every time, so it cannot play a Steam that was running and then quit: a test
/// that seeded it as running would sit through the whole shutdown timeout and
/// then fail.
fn stopped_steam() -> MockRunner {
    MockRunner::new()
        .failing("pgrep -x steam")
        .with_file(CONFIG, written_config())
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("journal")
}

fn plan_of(step: ImprovementId, undo: Undo) -> RollbackPlan {
    RollbackPlan {
        run: RunId::generate(),
        undos: vec![PlannedUndo { step, seq: 1, undo }],
    }
}

/// Puts the pin back to what Steam had, which was nothing.
fn restore_config() -> Undo {
    Undo::RestoreSteamConfig {
        path: CONFIG.into(),
        sections: vec![PriorSection {
            section: [
                "InstallConfigStore",
                "Software",
                "Valve",
                "Steam",
                "CompatToolMapping",
                "1422450",
            ]
            .iter()
            .map(|part| (*part).to_owned())
            .collect(),
            prior: PriorBlock::Present {
                entries: vec![PriorScalar {
                    key: "name".to_owned(),
                    value: Some("proton_experimental".to_owned()),
                }],
            },
        }],
    }
}

fn sysctl_undo() -> Undo {
    Undo::SetSysctl {
        key: "vm.max_map_count".to_owned(),
        value: "1048576".to_owned(),
    }
}

#[test]
fn undoing_a_proton_pin_checks_whether_steam_is_running() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let plan = plan_of(SteamProton::id_const(), restore_config());

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir)).expect("rolled back");

    assert!(
        runner
            .commands()
            .iter()
            .any(|command| command == "pgrep -x steam"),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn undoing_launch_options_checks_whether_steam_is_running() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let plan = plan_of(SteamLaunchOptions::id_const(), restore_config());

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir)).expect("rolled back");

    assert!(
        runner
            .commands()
            .iter()
            .any(|command| command == "pgrep -x steam"),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn undoing_a_kernel_setting_leaves_steam_alone() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let plan = plan_of(
        ImprovementId::from_static("core.sysctl.max-map-count"),
        sysctl_undo(),
    );

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir)).expect("rolled back");

    assert!(
        !runner
            .commands()
            .iter()
            .any(|command| command.contains("steam")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn a_steam_that_is_already_stopped_is_not_asked_to_quit() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let plan = plan_of(SteamProton::id_const(), restore_config());

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir)).expect("rolled back");

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
fn the_recorded_settings_still_go_back_into_the_written_config() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let plan = plan_of(SteamProton::id_const(), restore_config());

    let report = undo_with_steam_closed(&plan, &runner, &mut journal(&dir)).expect("rolled back");

    let after = runner.file(CONFIG).expect("still there");
    assert!(after.contains("proton_experimental"), "{after}");
    assert!(!after.contains("GE-Proton11-3"), "{after}");
    assert_eq!(report.reverted(), 1);
    assert!(matches!(
        report.undos[0].outcome,
        UndoOutcome::Reverted { .. }
    ));
}
