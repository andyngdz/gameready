//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

/// The label that introduces the command that puts things back.
pub(crate) const UNDO: &str = "Undo";

mod doctor;
mod explain;
mod games;
mod governor;
mod help;
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
pub use help::HelpCard;
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
