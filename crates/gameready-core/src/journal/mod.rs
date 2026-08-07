//! The undo record.
//!
//! Every mutation gameready performs is described here and made durable before
//! it happens. Rollback replays these in reverse.

mod domain;
mod errors;
mod service;

pub use domain::{
    digest, Change, JournalEvent, JournalRecord, PriorUnitState, RunId, Undo, JOURNAL_VERSION,
};
pub use errors::JournalError;
pub use service::{load, Journal, StatePaths};
