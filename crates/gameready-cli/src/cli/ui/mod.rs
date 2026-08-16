//! Turning reports into printable views. Nothing here writes to stdout; `main`
//! does.

/// The label that introduces the command that puts things back.
pub(crate) const UNDO: &str = "Undo";

/// What the tunings that are about the machine itself are called.
pub(crate) const SYSTEM: &str = "System";

/// What the tunings that are about one game are called.
pub(crate) const PER_GAME: &str = "Per game";

/// What to call `count` tunings.
///
/// Shared so the screen that counts them while probing and the screen that
/// counts them in the plan cannot end up calling them different things.
pub(crate) const fn tunings(count: usize) -> &'static str {
    if count == 1 {
        "tuning"
    } else {
        "tunings"
    }
}

/// What to call `count` games, for the questions that count them.
pub(crate) const fn games_noun(count: usize) -> &'static str {
    if count == 1 {
        "game"
    } else {
        "games"
    }
}

mod answers;
mod doctor;
mod explain;
mod games;
mod governor;
mod help;
mod init;
mod install;
mod launch_choice;
pub(crate) mod layout;
mod looking;
mod names;
mod overlay;
mod plan;
mod progress;
mod prompt;
mod proton_choice;
pub(crate) mod questions;
mod region;
mod rollback;
mod rows;
mod selftest;
mod steps;
mod summary;
mod takeover;
mod theme;

pub use answers::{ask_everything, Answers};
pub use doctor::DoctorReport;
pub use explain::{StepExplanation, StepIndex};
pub use games::GameList;
pub use governor::choose_governor_persistence;
pub use help::HelpCard;
pub use init::LaunchInstructions;
pub use install::{consent_to_install, InstallList};
pub use launch_choice::{choose_how_to_apply, LaunchChoice, SteamSettingsDone, SteamWork};
pub use looking::{LookingAtMachine, SteamGames};
pub use names::{name_column, short_names, widest};
pub use overlay::choose_overlay;
pub use plan::InitPlan;
pub use progress::ProgressView;
pub use prompt::choose_games;
pub use proton_choice::{choose_proton_pin, ProtonPin};
pub use questions::{Picker, Questions};
pub use rollback::{confirm_steam_close, RollbackSummary};
pub use selftest::SelftestSummary;
pub use steps::Steps;
pub use summary::Summary;
pub use takeover::{ask_takeovers, choose_takeover};
