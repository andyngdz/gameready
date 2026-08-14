//! Appending to and reading back the undo journal.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::journal::domain::{JournalEvent, JournalRecord, RunId};
use crate::journal::errors::JournalError;

/// Where gameready keeps everything it needs to undo its own work.
///
/// Under `$XDG_STATE_HOME` rather than `/var/lib` on purpose. The process runs
/// unprivileged and must be able to make the undo record durable *before* it
/// escalates to change anything. Needing root to write the journal would open a
/// window where the system is changed and the record is not yet on disk, which
/// is the exact failure this design exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePaths {
    root: PathBuf,
}

impl StatePaths {
    /// Uses `root` as the state directory. The caller resolves it, so this
    /// crate reads no environment of its own.
    #[must_use]
    pub const fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// The state directory itself.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The append-only journal. Never pruned: it is state, not diagnostics.
    #[must_use]
    pub fn journal(&self) -> PathBuf {
        self.root.join("journal.jsonl")
    }

    /// Where a run's summary is filed, one file per run.
    #[must_use]
    pub fn runs(&self) -> PathBuf {
        self.root.join("runs")
    }

    /// Log files, pruned by age and count at startup.
    #[must_use]
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Creates every directory the run will write into.
    pub fn ensure(&self) -> Result<(), JournalError> {
        for dir in [self.root.clone(), self.runs(), self.logs()] {
            std::fs::create_dir_all(&dir)
                .map_err(|source| JournalError::StateDir { path: dir, source })?;
        }
        Ok(())
    }
}

/// The append-only undo log for one run.
///
/// Every append is fsync'd before returning. That is deliberately the slow
/// choice: a run performs tens of mutations, not thousands, and an unsynced
/// undo record does not survive the power loss it exists to protect against.
#[derive(Debug)]
pub struct Journal {
    paths: StatePaths,
    run: RunId,
    file: File,
    seq: u64,
}

impl Journal {
    /// Opens the journal for a run, creating the state directories if needed.
    pub fn open(paths: StatePaths, run: RunId) -> Result<Self, JournalError> {
        paths.ensure()?;
        let path = paths.journal();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| JournalError::Open {
                path: path.clone(),
                source,
            })?;

        Ok(Self {
            paths,
            run,
            file,
            seq: 0,
        })
    }

    /// Which run this journal is recording.
    #[must_use]
    pub const fn run(&self) -> RunId {
        self.run
    }

    /// Where this journal's state lives.
    #[must_use]
    pub const fn paths(&self) -> &StatePaths {
        &self.paths
    }

    /// How many records have been written for this run.
    #[must_use]
    pub const fn seq(&self) -> u64 {
        self.seq
    }

    /// Appends one event and flushes it to disk before returning.
    ///
    /// The caller must not perform the mutation the event describes until this
    /// has returned successfully.
    pub fn append(&mut self, event: JournalEvent) -> Result<u64, JournalError> {
        let seq = self.seq;
        let record = JournalRecord::new(self.run, seq, event);
        let path = self.paths.journal();

        let mut line = serde_json::to_vec(&record).map_err(|source| JournalError::Corrupt {
            path: path.clone(),
            line: seq as usize,
            source,
        })?;
        line.push(b'\n');

        self.file
            .write_all(&line)
            .map_err(|source| JournalError::Append {
                path: path.clone(),
                source,
            })?;

        self.file
            .sync_data()
            .map_err(|source| JournalError::Sync { path, source })?;

        self.seq += 1;
        Ok(seq)
    }
}

/// Reads every record in a journal file.
///
/// A corrupt line stops the read rather than being skipped: a journal with a
/// hole in it cannot be replayed safely, and silently ignoring the hole would
/// produce a rollback that misses a change.
///
/// An unparseable *final* line is the one exception, because it is not a hole.
/// `append` writes each record and its newline in a single `write_all`, so a
/// process killed partway through leaves an unterminated fragment at the end of
/// the file and nothing else. Every record before it was fsynced and is intact.
/// Treating that fragment as whole-file corruption would make every completed
/// run in the journal unrollbackable, which is the opposite of what this file
/// exists for. Dropping it loses nothing either: the record is fsynced before
/// the mutation it describes is allowed to run, so a change whose record never
/// finished being written never happened.
pub fn load(path: &Path) -> Result<Vec<JournalRecord>, JournalError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(JournalError::Open {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let mut records = Vec::new();
    // split_inclusive keeps the newline, which is what tells a completed record
    // apart from the tail of a write that never finished.
    for (index, line) in bytes.split_inclusive(|byte| *byte == b'\n').enumerate() {
        let terminated = line.last() == Some(&b'\n');
        let text = line.strip_suffix(b"\n").unwrap_or(line);
        if text.iter().all(u8::is_ascii_whitespace) {
            continue;
        }

        match serde_json::from_slice(text) {
            Ok(record) => records.push(record),
            Err(_) if !terminated => break,
            Err(source) => {
                return Err(JournalError::Corrupt {
                    path: path.to_path_buf(),
                    line: index + 1,
                    source,
                });
            }
        }
    }

    Ok(records)
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
