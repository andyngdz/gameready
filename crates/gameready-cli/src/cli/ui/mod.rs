//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

/// The line an empty list prints, so a bare heading never looks like a bug.
pub(crate) const NOTHING: &str = "  none";

/// The label that introduces the command that puts things back.
pub(crate) const UNDO: &str = "Undo";

/// The heading over a list of steps, wherever one is printed.
pub(crate) const STEPS: &str = "Steps";

mod doctor;
mod explain;
mod games;
mod governor;
mod init;
mod install;
mod launch_choice;
mod launch_report;
pub(crate) mod layout;
mod overlay;
mod plan;
mod progress;
mod prompt;
pub(crate) mod questions;
mod rollback;
mod selftest;
mod summary;

pub use doctor::{DoctorReport, StepFinding};
pub use explain::{StepExplanation, StepIndex};
pub use games::GameList;
pub use governor::choose_governor_persistence;
pub use init::LaunchInstructions;
pub use install::{consent_to_install, InstallList};
pub use launch_choice::{choose_how_to_apply, LaunchChoice, SteamWork};
pub use launch_report::LaunchReport;
pub use overlay::choose_overlay;
pub use plan::InitPlan;
pub use progress::ProgressView;
pub use prompt::choose_games;
pub use questions::{ask_everything, Answers, Picker, Questions};
pub use rollback::RollbackSummary;
pub use selftest::SelftestSummary;
pub use summary::Summary;
