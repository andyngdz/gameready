//! The undo record.
//!
//! Every mutation gameready performs is described here and made durable before
//! it happens. Rollback replays these in reverse.

mod domain;
mod errors;
mod service;

pub use domain::{Change, JOURNAL_VERSION, JournalEvent, JournalRecord, RunId, Undo};
pub use errors::JournalError;
pub use service::{Journal, StatePaths, load};
