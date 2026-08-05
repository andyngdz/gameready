//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

mod games;
mod rollback;
mod selftest;
mod summary;

pub use games::GameList;
pub use rollback::RollbackSummary;
pub use selftest::SelftestSummary;
pub use summary::Summary;
