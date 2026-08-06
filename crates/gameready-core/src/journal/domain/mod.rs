//! What the journal records.

pub(crate) mod change;
mod record;
mod undo;

pub use change::{Change, digest};
pub use record::{JOURNAL_VERSION, JournalEvent, JournalRecord, RunId};
pub use undo::{PriorUnitState, Undo};
