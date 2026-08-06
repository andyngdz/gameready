//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

/// The line an empty list prints, so a bare heading never looks like a bug.
pub(crate) const NOTHING: &str = "  none";

pub(crate) mod colors;
mod games;
mod init;
mod launch_choice;
mod launch_report;
mod overlay;
mod plan;
mod progress;
mod prompt;
pub(crate) mod questions;
mod rollback;
mod selftest;
mod summary;

pub use games::GameList;
pub use init::LaunchInstructions;
pub use launch_choice::{LaunchChoice, choose_how_to_apply};
pub use launch_report::LaunchReport;
pub use overlay::choose_overlay;
pub use plan::InitPlan;
pub use progress::ProgressView;
pub use prompt::choose_games;
pub use questions::{Picker, ask_everything};
pub use rollback::RollbackSummary;
pub use selftest::SelftestSummary;
pub use summary::Summary;
