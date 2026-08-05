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
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Report system facts and what each step would do.
    Doctor,

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
