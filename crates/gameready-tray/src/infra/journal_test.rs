use std::fs::OpenOptions;
use std::io::Write as _;
use std::sync::mpsc;
use std::thread;

use tempfile::TempDir;

use super::*;

/// Appends to a file in `dir`, the way a run appends to its journal.
fn append(dir: &std::path::Path, name: &str) {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))
        .expect("the file should open");
    writeln!(file, "a step ran").expect("the write should land");
    file.sync_all().expect("the fsync should land");
}

#[test]
fn a_write_to_the_journal_wakes_the_watcher() {
    let state = TempDir::new().expect("a temp dir");
    let root = state.path().to_path_buf();
    let (woken, wakes) = mpsc::channel();

    let watching = thread::spawn(move || {
        // One wake is enough: returning false is how the caller says stop.
        let _ = watch_journal(&root, || {
            woken.send(()).ok();
            false
        });
    });

    // Written repeatedly: the watcher registers its watch on its own thread, and
    // a write that lands first is a write inotify never saw. The journal does
    // not exist yet either, which is the state of a machine that has never had
    // a run and the reason the watch is on the directory.
    let woken = (0..50).any(|_| {
        append(state.path(), JOURNAL);
        wakes
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_ok()
    });

    assert!(woken, "a journal write should wake the watcher");
    watching.join().expect("the watcher should stop");
}

#[test]
fn a_write_to_anything_else_in_the_state_dir_is_ignored() {
    let state = TempDir::new().expect("a temp dir");
    let root = state.path().to_path_buf();
    let (woken, wakes) = mpsc::channel();

    let watching = thread::spawn(move || {
        let _ = watch_journal(&root, || {
            woken.send(()).ok();
            false
        });
    });

    // Backups and logs land in the same directory and change nothing the menu
    // shows, so waking for them would re-probe the machine for no reason.
    for _ in 0..5 {
        append(state.path(), "something-else.log");
        assert!(wakes
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err());
    }

    // Stop the thread by giving it the one write it does wake for.
    while wakes
        .recv_timeout(std::time::Duration::from_millis(100))
        .is_err()
    {
        append(state.path(), JOURNAL);
    }
    watching.join().expect("the watcher should stop");
}

#[test]
fn a_state_directory_that_is_not_there_is_reported_rather_than_hung_on() {
    let missing = TempDir::new().expect("a temp dir");
    let path = missing.path().join("never-created");

    assert!(watch_journal(&path, || true).is_err());
}
