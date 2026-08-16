//! Undoing a previous run.

mod domain;
mod errors;
mod service;

pub use domain::{PlannedUndo, RollbackPlan, RollbackReport, UndoOutcome, UndoReport};
pub use errors::RollbackError;
pub use service::{changes_for, execute, latest_run, plan};
