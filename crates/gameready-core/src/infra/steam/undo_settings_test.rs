use tempfile::TempDir;

use super::*;
use crate::improvement::{ImprovementId, Privilege};
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths, Undo};
use crate::rollback::{PlannedUndo, UndoOutcome};

const CONFIG: &str = "/steam/config/config.vdf";
const BACKUP: &str = "/state/backups/01/config.vdf";

/// Every test seeds Steam as stopped. `MockRunner` answers `pgrep` the same way
/// every time, so it cannot play a Steam that was running and then quit: a test
/// that seeded it as running would sit through the whole shutdown timeout and
/// then fail.
fn stopped_steam() -> MockRunner {
    MockRunner::new()
        .failing("pgrep -x steam")
        .with_file(BACKUP, "the config Steam had")
        .with_file(CONFIG, "the config gameready wrote")
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

fn restore_config() -> Undo {
    Undo::RestoreFile {
        path: CONFIG.into(),
        from: BACKUP.into(),
        mode: 0o644,
        privilege: Privilege::User,
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

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir), PackagePolicy::Keep)
        .expect("rolled back");

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

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir), PackagePolicy::Keep)
        .expect("rolled back");

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

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir), PackagePolicy::Keep)
        .expect("rolled back");

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

    undo_with_steam_closed(&plan, &runner, &mut journal(&dir), PackagePolicy::Keep)
        .expect("rolled back");

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
fn the_backup_still_goes_back_over_the_written_config() {
    let dir = TempDir::new().expect("temp dir");
    let runner = stopped_steam();
    let plan = plan_of(SteamProton::id_const(), restore_config());

    let report = undo_with_steam_closed(&plan, &runner, &mut journal(&dir), PackagePolicy::Keep)
        .expect("rolled back");

    assert_eq!(runner.file(CONFIG).as_deref(), Some("the config Steam had"));
    assert_eq!(report.reverted(), 1);
    assert!(matches!(
        report.undos[0].outcome,
        UndoOutcome::Reverted { .. }
    ));
}
