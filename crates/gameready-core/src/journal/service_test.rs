use tempfile::TempDir;

use super::*;
use crate::improvement::ImprovementId;
use crate::journal::Change;

fn paths(dir: &TempDir) -> StatePaths {
    StatePaths::new(dir.path().to_path_buf())
}

#[test]
fn an_absent_journal_reads_as_empty_rather_than_failing() {
    let dir = TempDir::new().expect("temp dir");
    let records = load(&paths(&dir).journal()).expect("absent is not an error");
    assert!(records.is_empty());
}

#[test]
fn appended_records_read_back_in_order() {
    let dir = TempDir::new().expect("temp dir");
    let run = RunId::generate();
    let mut journal = Journal::open(paths(&dir), run).expect("opens");
    let step = ImprovementId::from_static("core.sysctl.max-map-count");

    journal
        .append(JournalEvent::RunBegin {
            argv: vec!["gameready".to_owned(), "apply".to_owned()],
            tool_version: "0.1.0".to_owned(),
        })
        .expect("appends");
    journal
        .append(JournalEvent::StepBegin { step: step.clone() })
        .expect("appends");
    journal
        .append(JournalEvent::Changed {
            step,
            change: Change::SysctlRuntime {
                key: "vm.max_map_count".to_owned(),
                previous: "1048576".to_owned(),
            },
        })
        .expect("appends");

    let records = load(&paths(&dir).journal()).expect("reads");
    assert_eq!(records.len(), 3);
    assert_eq!(records[0].seq, 0);
    assert_eq!(records[2].seq, 2);
    assert!(records.iter().all(|record| record.run == run));
}

#[test]
fn opening_creates_the_directories_a_run_writes_into() {
    let dir = TempDir::new().expect("temp dir");
    let run = RunId::generate();
    let state = paths(&dir);
    let _journal = Journal::open(state.clone(), run).expect("opens");

    assert!(state.runs().is_dir());
    assert!(state.backups(run).is_dir());
    assert!(state.logs().is_dir());
}

#[test]
fn a_run_id_round_trips_through_its_text_form() {
    let run = RunId::generate();
    let parsed = RunId::parse(&run.to_string()).expect("round trips");
    assert_eq!(parsed, run);
}

#[test]
fn a_corrupt_line_stops_the_read_rather_than_being_skipped() {
    // A journal with a hole cannot be replayed safely; silently skipping the
    // hole would produce a rollback that misses a change.
    let dir = TempDir::new().expect("temp dir");
    let journal_path = paths(&dir).journal();
    std::fs::create_dir_all(dir.path()).expect("dir");
    std::fs::write(&journal_path, "{\"not\": \"a record\"}\n").expect("write");

    assert!(load(&journal_path).is_err());
}
