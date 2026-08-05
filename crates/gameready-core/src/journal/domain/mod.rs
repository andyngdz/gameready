//! What the journal records.

pub(crate) mod change;
mod record;

pub use change::{Change, Undo};
pub use record::{JOURNAL_VERSION, JournalEvent, JournalRecord, RunId};
