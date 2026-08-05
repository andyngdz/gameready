//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

mod rollback;
mod summary;

pub use rollback::RollbackSummary;
pub use summary::Summary;
