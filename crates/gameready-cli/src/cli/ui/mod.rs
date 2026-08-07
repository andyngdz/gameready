//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

/// The line an empty list prints, so a bare heading never looks like a bug.
pub(crate) const NOTHING: &str = "  none";

/// The label that introduces the command that puts things back.
pub(crate) const UNDO: &str = "Undo";

pub(crate) mod colors;
mod explain;
mod games;
mod init;
mod install;
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

pub use explain::{StepExplanation, StepIndex};
pub use games::GameList;
pub use init::LaunchInstructions;
pub use install::{InstallList, consent_to_install};
pub use launch_choice::{LaunchChoice, SteamWork, choose_how_to_apply};
pub use launch_report::LaunchReport;
pub use overlay::choose_overlay;
pub use plan::InitPlan;
pub use progress::ProgressView;
pub use prompt::choose_games;
pub use questions::{Answers, Picker, Questions, ask_everything};
pub use rollback::RollbackSummary;
pub use selftest::SelftestSummary;
pub use summary::Summary;
