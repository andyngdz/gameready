//! What the journal records.

pub(crate) mod change;
mod record;

pub use change::{Change, PriorUnitState, Undo, digest};
pub use record::{JOURNAL_VERSION, JournalEvent, JournalRecord, RunId};
