//! Command line surface.

use clap::{CommandFactory as _, FromArgMatches as _, Parser, Subcommand};

use crate::cli::ui::{HelpCard, Picker};
use gameready_core::run::Mode;

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

impl Cli {
    /// Parses the command line, with the hand-written help card standing in for
    /// the flat list clap generates.
    ///
    /// Clap groups flags under headings but has no such thing for subcommands,
    /// and the grouping is the whole point of the card: it answers "which one
    /// do I type" rather than listing eight equally likely commands. Each
    /// subcommand's own `--help` is still clap's, which is where the global
    /// flags stay documented.
    #[must_use]
    pub fn parsed() -> Self {
        let matches = Self::command()
            .override_help(HelpCard.to_string())
            .get_matches();
        Self::from_arg_matches(&matches).unwrap_or_else(|error| error.exit())
    }
}

/// Whether a command will change the system.
///
/// Drives the one credential prompt at the start of a run. Every privileged
/// command runs with `-n` afterwards, so a command that mutates without priming
/// first fails against a cold cache rather than asking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Only reads. Runs without an escalator at all if the machine has none.
    Reads,
    /// Changes something, so the credential cache is primed first.
    Mutates,
}

impl Command {
    /// Whether this command will change the system.
    #[must_use]
    pub const fn effect(&self) -> Effect {
        match self {
            Self::Doctor | Self::Explain { .. } | Self::ListGames => Effect::Reads,
            Self::Init { dry_run, .. } | Self::Apply { dry_run, .. } => {
                if *dry_run {
                    Effect::Reads
                } else {
                    Effect::Mutates
                }
            }
            Self::Rollback { .. } | Self::Selftest { .. } => Effect::Mutates,
        }
    }

    /// Whether there is anyone at the terminal to answer this run's questions.
    ///
    /// `--yes` is the caller answering in advance, which is the only way a run
    /// in a script or a pipe gets past a question nobody can be asked.
    #[must_use]
    pub const fn picker(&self) -> Picker {
        match self {
            Self::Init { yes: true, .. } | Self::Apply { yes: true, .. } => Picker::TakeAll,
            Self::Init { .. }
            | Self::Apply { .. }
            | Self::Doctor
            | Self::Explain { .. }
            | Self::ListGames
            | Self::Rollback { .. }
            | Self::Selftest { .. } => Picker::Ask,
        }
    }

    /// Whether this command should compute the plan or carry it out.
    ///
    /// Commands with no `--dry-run` of their own always apply; the ones that
    /// have the flag answer with it.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        match self {
            Self::Init { dry_run, .. } | Self::Apply { dry_run, .. } => {
                if *dry_run {
                    Mode::DryRun
                } else {
                    Mode::Apply
                }
            }
            Self::Doctor
            | Self::Explain { .. }
            | Self::ListGames
            | Self::Rollback { .. }
            | Self::Selftest { .. } => Mode::Apply,
        }
    }
}

/// The subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Set up this machine: find your games, pick which to tune, apply.
    ///
    /// The one command most people need. Everything it does is undoable with
    /// `gameready rollback`.
    Init {
        /// Answer every question in advance: take every installed game, and
        /// install whatever the steps need without asking.
        ///
        /// For scripts and terminals that cannot prompt. Installed packages
        /// stay behind after `gameready rollback`, so this agrees to something
        /// you cannot fully undo.
        #[arg(long)]
        yes: bool,

        /// Show a frame-rate overlay in game, without being asked.
        ///
        /// Off unless given. With `--yes` there is nobody to ask, so this is
        /// the only way a scripted run turns the overlay on.
        #[arg(long)]
        fps_overlay: bool,

        /// Work out what would change without changing anything.
        #[arg(long)]
        dry_run: bool,
    },

    /// Report system facts and what each step would do.
    Doctor,

    /// Say what a step does, why, and what it would change on this machine.
    ///
    /// Reads only. Run it before `apply --step <id>` to see the same step's
    /// plan without agreeing to any of it.
    Explain {
        /// Which step, by id. Leave it out to list them.
        step: Option<String>,
    },

    /// List the game profiles gameready can see, and where each came from.
    ListGames,

    /// Apply improvements.
    Apply {
        /// Apply only this step, by id.
        #[arg(long)]
        step: Option<String>,

        /// Install whatever the steps need without asking.
        ///
        /// For scripts and terminals that cannot prompt. Installed packages
        /// stay behind after `gameready rollback`, so this agrees to something
        /// you cannot fully undo.
        #[arg(long)]
        yes: bool,

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

#[cfg(test)]
#[path = "args_test.rs"]
mod args_test;
