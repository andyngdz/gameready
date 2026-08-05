//! Wiring: parse arguments, build the runner and state paths, dispatch, exit.

// A test reports failure by panicking, so expect, unwrap, and panic are its
// assertion mechanism. The deny in Cargo.toml targets the paths that run on a
// user's machine, where a panic would abandon a half-applied change.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]

mod cli;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use clap::Parser;
use directories::ProjectDirs;
use gameready_core::infra::exec::RealRunner;
use gameready_core::journal::StatePaths;
use gameready_core::rollback::PackagePolicy;
use gameready_core::run::{Mode, RunStatus};

use crate::cli::args::{Cli, Command};

/// The name every per-user directory is built from. Named once because the
/// state directory and the config directory both derive from it, and two copies
/// would let one be renamed without the other.
const PROJECT: &str = "gameready";

fn main() -> ExitCode {
    let cli = Cli::parse();

    match dispatch(&cli) {
        Ok((status, output)) => {
            print!("{output}");
            ExitCode::from(u8::try_from(status.code()).unwrap_or(1))
        }
        Err(error) => {
            eprintln!("gameready: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(cli: &Cli) -> Result<(RunStatus, String)> {
    let paths = state_paths(cli.state_dir.clone())?;

    let runner = if cli.command.mutates() {
        let runner = RealRunner::detect().context("no way to run privileged commands was found")?;
        runner
            .prime()
            .context("could not get permission to make system changes")?;
        runner
    } else {
        RealRunner::detect().unwrap_or_else(|_| RealRunner::unprivileged())
    };

    match &cli.command {
        Command::Doctor => Ok((RunStatus::Clean, cli::commands::doctor(&runner)?)),

        Command::ListGames => {
            let games = user_games_dir(cli.games_dir.clone())?;
            Ok((RunStatus::Clean, cli::commands::list_games(&games)?))
        }

        Command::Apply { step, dry_run } => {
            let mode = if *dry_run { Mode::DryRun } else { Mode::Apply };
            let (report, rendered) = cli::commands::apply(&runner, paths, step.as_deref(), mode)?;
            let output = if cli.json {
                serde_json::to_string_pretty(&report)? + "\n"
            } else {
                rendered
            };
            Ok((report.status(), output))
        }

        Command::Rollback {
            run,
            purge_packages,
        } => {
            let policy = if *purge_packages {
                PackagePolicy::Purge
            } else {
                PackagePolicy::Keep
            };
            cli::commands::rollback(&runner, paths, run.as_deref(), policy)
        }

        Command::Selftest { step: _ } => cli::commands::selftest(&runner, paths),
    }
}

/// Resolves where the user's own game profiles live.
///
/// Separate from the state directory: profiles are configuration a user writes
/// and keeps, while the state directory is data gameready writes and prunes.
fn user_games_dir(override_dir: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = override_dir {
        return Ok(dir);
    }
    let dirs = ProjectDirs::from("", "", PROJECT)
        .context("could not determine a config directory for this user")?;
    Ok(dirs.config_dir().join("games"))
}

/// Resolves where the journal, backups, and logs live.
fn state_paths(override_dir: Option<PathBuf>) -> Result<StatePaths> {
    if let Some(dir) = override_dir {
        return Ok(StatePaths::new(dir));
    }
    let dirs = ProjectDirs::from("", "", PROJECT)
        .context("could not determine a state directory for this user")?;
    let root = dirs.state_dir().unwrap_or_else(|| dirs.data_dir());
    Ok(StatePaths::new(root.to_path_buf()))
}
