use gameready_core::improvement::ImprovementId;
use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::{StatePaths, Undo};
use gameready_core::rollback::PlannedUndo;

use super::*;

#[test]
fn a_file_undo_becomes_a_removal_preview_row() {
    let planned = PlannedUndo {
        step: ImprovementId::from_static("core.gamemode.config"),
        seq: 1,
        undo: Undo::DeleteFile {
            path: "/home/tester/.config/gamemode.ini".into(),
            expect_sha256: "hash".to_owned(),
            privilege: gameready_core::improvement::Privilege::User,
        },
    };

    let rows = preview_rows(&planned, &MockRunner::new());

    assert_eq!(rows.len(), 1);
    let first = &rows[0];
    assert_eq!(first.subject, "gamemode.ini");
    assert_eq!(first.evidence, "remove /home/tester/.config/gamemode.ini");
}

#[test]
fn an_empty_journal_has_no_rollback_plan() {
    let directory = tempfile::tempdir().expect("temp directory");
    let paths = StatePaths::new(directory.path().to_path_buf());

    let error = rollback_plan(&paths, None).expect_err("there is no recorded run");

    assert!(error.to_string().contains("no runs"), "{error}");
}
