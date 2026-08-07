//! What the journal records.

pub(crate) mod change;
mod record;
mod undo;

pub use change::{digest, Change};
pub use record::{JournalEvent, JournalRecord, RunId, JOURNAL_VERSION};
pub use undo::{PriorUnitState, Undo};
