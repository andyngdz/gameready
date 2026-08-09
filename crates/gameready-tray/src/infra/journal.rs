//! Noticing when a gameready run has changed something.
//!
//! The journal is the record of every change: a step appends to it and fsyncs
//! before returning, so a write landing there is the one signal that says the
//! machine is not what the tray last read. Watching it beats asking the machine
//! on a timer, and beats having the CLI announce itself: nothing needs adding
//! to the CLI, and a run under `sudo` in another terminal is caught the same as
//! one in this session.

use std::mem::MaybeUninit;
use std::path::Path;

use rustix::fs::inotify::{self, CreateFlags, WatchFlags};

use crate::infra::errors::WatchError;

/// The file inside the state directory that records every change.
const JOURNAL: &str = "journal.jsonl";

/// Room for a batch of events. Names here are one short filename, so this holds
/// many events per read and the size is not worth tuning.
const BUFFER: usize = 4096;

/// Blocks until a gameready run writes to the journal, and reports each time.
///
/// Watches the directory rather than the file: on a machine that has never had
/// a run there is no journal yet, and a watch on a path that does not exist
/// fails. The directory watch catches its creation and every append after.
///
/// Runs until `changed` returns `false`, which is how the caller says it has
/// gone away.
pub fn watch_journal(state: &Path, mut changed: impl FnMut() -> bool) -> Result<(), WatchError> {
    let inotify = inotify::init(CreateFlags::CLOEXEC)?;
    inotify::add_watch(
        &inotify,
        state,
        WatchFlags::MODIFY | WatchFlags::CREATE | WatchFlags::MOVED_TO,
    )?;

    let mut buffer = [MaybeUninit::uninit(); BUFFER];
    loop {
        let mut reader = inotify::Reader::new(&inotify, &mut buffer);
        // One sweep per batch, not per event: a run appends a line per step and
        // the tray would otherwise re-probe the machine a dozen times over.
        let mut touched = false;
        loop {
            match reader.next() {
                Ok(event) => {
                    touched |= event
                        .file_name()
                        .is_some_and(|name| name.to_bytes() == JOURNAL.as_bytes());
                }
                Err(rustix::io::Errno::AGAIN | rustix::io::Errno::INTR) => break,
                Err(error) => return Err(WatchError::Watch(error)),
            }
            if reader.is_buffer_empty() {
                break;
            }
        }
        if touched && !changed() {
            return Ok(());
        }
    }
}

#[cfg(test)]
#[path = "journal_test.rs"]
mod journal_test;
