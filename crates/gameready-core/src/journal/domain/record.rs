//! The records that make up the journal.

use std::fmt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::improvement::ImprovementId;
use crate::journal::domain::change::Change;

/// Identifies one invocation of gameready.
///
/// A ULID rather than a counter: runs are appended from a per-user file with no
/// coordination, and a monotonic-by-time id sorts correctly without needing to
/// read what came before.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(Ulid);

impl RunId {
    /// A fresh id for a run starting now.
    #[must_use]
    pub fn generate() -> Self {
        Self(Ulid::generate())
    }

    /// Parses an id a user passed to `--run`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Ulid::from_string(text).ok().map(Self)
    }

    /// When the run started, read from the timestamp a ULID carries in its
    /// leading bits. This is why the id is a ULID rather than a counter: the
    /// time is in the id, so `rollback` can name a run by when it happened
    /// without a separate clock field to keep in sync.
    #[must_use]
    pub fn started_at(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.0.timestamp_ms())
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The journal format version, bumped when a record shape changes
/// incompatibly. Present on every line so an old journal stays readable.
pub const JOURNAL_VERSION: u32 = 1;

/// One line of the journal.
///
/// Append-only and never rewritten. `seq` orders records within a run, which is
/// what rollback replays in reverse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalRecord {
    /// Format version, so a reader can refuse a journal it does not understand.
    pub v: u32,

    /// Which run this belongs to.
    pub run: RunId,

    /// Position within the run, starting at zero.
    pub seq: u64,

    /// What happened.
    pub event: JournalEvent,
}

impl JournalRecord {
    /// Builds a record at the current format version.
    #[must_use]
    pub const fn new(run: RunId, seq: u64, event: JournalEvent) -> Self {
        Self {
            v: JOURNAL_VERSION,
            run,
            seq,
            event,
        }
    }
}

/// What a journal line records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalEvent {
    /// A run started. A run with this and no `RunEnd` was interrupted, which is
    /// how `status` finds work needing recovery.
    RunBegin {
        argv: Vec<String>,
        tool_version: String,
    },

    /// A step is about to apply.
    StepBegin { step: ImprovementId },

    /// A mutation is about to be performed. Written and fsync'd first; the
    /// mutation happens after this line is durable.
    Changed { step: ImprovementId, change: Change },

    /// A step finished, with how it ended.
    StepEnd {
        step: ImprovementId,
        outcome: String,
    },

    /// A run finished cleanly.
    RunEnd {
        applied: usize,
        skipped: usize,
        failed: usize,
    },

    /// A rollback of a prior run started.
    RollbackBegin { target: RunId },

    /// One undo was performed.
    Undone { step: ImprovementId, detail: String },

    /// A rollback finished.
    RollbackEnd { undone: usize, failed: usize },
}
