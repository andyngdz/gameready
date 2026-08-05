//! Command line surface.

use clap::{Parser, Subcommand};

/// Apply gaming-related system tuning on Linux, and undo it.
#[derive(Debug, Parser)]
#[command(name = "gameready", version, about, long_about = None)]
pub struct Cli {
    /// What to do.
    #[command(subcommand)]
    pub command: Command,

    /// Print the run report as JSON instead of a rendered summary.
    #[arg(long, global = true)]
    pub json: bool,

    /// Write the state directory somewhere else. Used by tests to keep runs
    /// out of the invoking user's real journal.
    #[arg(long, global = true, env = "GAMEREADY_STATE_DIR")]
    pub state_dir: Option<std::path::PathBuf>,

    /// Read your own game profiles from somewhere other than
    /// `~/.config/gameready/games`.
    #[arg(long, global = true, env = "GAMEREADY_GAMES_DIR")]
    pub games_dir: Option<std::path::PathBuf>,
}

impl Command {
    /// Whether this command will change the system.
    ///
    /// Drives the one credential prompt at the start of a run. Every
    /// privileged command runs with `-n` afterwards, so a command that mutates
    /// without priming first fails against a cold cache rather than asking.
    #[must_use]
    pub const fn mutates(&self) -> bool {
        match self {
            Self::Doctor | Self::ListGames => false,
            Self::Apply { dry_run, .. } => !*dry_run,
            Self::Rollback { .. } | Self::Selftest { .. } => true,
        }
    }
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Report system facts and what each step would do.
    Doctor,

    /// List the game profiles gameready can see, and where each came from.
    ListGames,

    /// Apply improvements.
    Apply {
        /// Apply only this step, by id.
        #[arg(long)]
        step: Option<String>,

        /// Compute the plan without changing anything.
        #[arg(long)]
        dry_run: bool,
    },

    /// Undo a previous run.
    Rollback {
        /// Which run to undo. Defaults to the most recent.
        #[arg(long)]
        run: Option<String>,

        /// Also remove packages the run installed.
        ///
        /// Off by default: uninstalling is not the inverse of installing, so
        /// the dependency cascade is the user's call, not ours.
        #[arg(long)]
        purge_packages: bool,
    },

    /// Apply a step, verify it, roll it back, and verify it reverted.
    ///
    /// The only way to prove a step that touches kernel state actually works,
    /// since containers cannot write `/proc/sys` and CI cannot repoint a
    /// scheduler.
    Selftest {
        /// Test only this step, by id.
        #[arg(long)]
        step: Option<String>,
    },
}

#[cfg(test)]
#[path = "args_test.rs"]
mod args_test;
