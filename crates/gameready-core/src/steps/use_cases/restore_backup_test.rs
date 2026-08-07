use std::path::PathBuf;

use tempfile::TempDir;

use super::*;
use crate::exec::ExecError;
use crate::facts::{Family, SystemFacts};
use crate::improvement::ImprovementId;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};

const CONFIG: &str = "/home/someone/.steam/steam/config/config.vdf";
const BACKUP: &str = "/state/backups/config.vdf";

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open")
}

fn written(backup: Option<PathBuf>) -> Change {
    Change::FileWritten {
        path: PathBuf::from(CONFIG),
        existed: true,
        backup,
        sha256_after: String::new(),
        mode: 0o644,
        privilege: Privilege::User,
    }
}

#[test]
fn a_file_comes_back_exactly_as_the_backup_holds_it() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .with_file(CONFIG, "changed".to_owned())
        .with_file(BACKUP, "original".to_owned());
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(
        cx,
        ImprovementId::from_static("test.restore"),
        &runner,
        &mut journal,
    );

    restore_from_backup(&[written(Some(PathBuf::from(BACKUP)))], &mut apply).expect("restored");

    assert_eq!(runner.file(CONFIG).as_deref(), Some("original"));
}

#[test]
fn a_write_with_no_backup_is_left_alone() {
    // Nothing was copied, so there is nothing to put back. Guessing at the
    // previous contents would be worse than leaving the file as it is.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_file(CONFIG, "changed".to_owned());
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(
        cx,
        ImprovementId::from_static("test.restore"),
        &runner,
        &mut journal,
    );

    restore_from_backup(&[written(None)], &mut apply).expect("restored");

    assert_eq!(runner.file(CONFIG).as_deref(), Some("changed"));
}

#[test]
fn a_missing_backup_file_is_reported_rather_than_swallowed() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_file(CONFIG, "changed".to_owned());
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(
        cx,
        ImprovementId::from_static("test.restore"),
        &runner,
        &mut journal,
    );

    let failure = restore_from_backup(&[written(Some(PathBuf::from(BACKUP)))], &mut apply);

    assert!(
        matches!(failure, Err(StepError::Exec(ExecError::Read { .. }))),
        "{failure:?}"
    );
}
