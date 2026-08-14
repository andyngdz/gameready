//! Wiring: parse arguments, build the runner and state paths, dispatch, exit.

// A test reports failure by panicking, so expect, unwrap, and panic are its
// assertion mechanism. The deny in Cargo.toml targets the paths that run on a
// user's machine, where a panic would abandon a half-applied change.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]

mod cli;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::infra::dirs;
use gameready_core::journal::StatePaths;
use gameready_core::run::{RunReport, RunStatus};
use gameready_core::steam::Overlay;

use crate::cli::args::{Cli, Command};
use crate::cli::escalation::Escalation;
use crate::cli::runtime::Machine;

fn main() -> ExitCode {
    let cli = Cli::parsed();

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
    let effect = cli.command.effect();
    let machine = Machine::detect(effect)?;

    // Built once for every command, so a command that changes the system cannot
    // reach its first `sudo -n` without having filled the credential cache.
    let authorize = || machine.authorize();
    let escalation = Escalation::for_effect(effect, &authorize);

    carry_out(cli, machine.runner(), paths, escalation)
}

/// Runs the command the flags named, against a machine already chosen.
fn carry_out(
    cli: &Cli,
    runner: &dyn CommandRunner,
    paths: StatePaths,
    escalation: Escalation<'_>,
) -> Result<(RunStatus, String)> {
    match &cli.command {
        Command::Init { fps_overlay, .. } => {
            reported(cli, init(cli, runner, paths, *fps_overlay, escalation)?)
        }

        Command::Doctor => Ok((RunStatus::Clean, cli::commands::doctor(runner)?)),

        Command::Explain { step } => Ok((
            RunStatus::Clean,
            cli::commands::explain(runner, step.as_deref())?,
        )),

        Command::ListGames => {
            let games = user_games_dir(cli.games_dir.clone())?;
            Ok((RunStatus::Clean, cli::commands::list_games(&games)?))
        }

        Command::Apply { step, .. } => reported(
            cli,
            cli::commands::apply(
                runner,
                paths,
                step.as_deref(),
                cli.command.mode(),
                cli.command.picker(),
                escalation,
            )?,
        ),

        Command::Rollback { run } => {
            cli::commands::rollback(runner, paths, run.as_deref(), escalation)
        }

        Command::Selftest { step } => {
            let games = user_games_dir(cli.games_dir.clone())?;
            cli::commands::selftest(runner, paths, step.as_deref(), &games, escalation)
        }
    }
}

/// Runs `init` with the picker and overlay the flags asked for.
///
/// The overlay flag only ever turns it on. Without it an interactive run asks,
/// and a run with nobody at the terminal leaves the screen alone.
fn init(
    cli: &Cli,
    runner: &dyn CommandRunner,
    paths: StatePaths,
    fps_overlay: bool,
    escalation: Escalation<'_>,
) -> Result<(RunReport, String)> {
    let games = user_games_dir(cli.games_dir.clone())?;
    let request = cli::commands::InitRequest {
        games_dir: &games,
        mode: cli.command.mode(),
        picker: cli.command.picker(),
        overlay: fps_overlay.then_some(Overlay::Show),
    };
    cli::commands::init(runner, paths, &request, escalation)
}

/// Renders a finished run as JSON or as the summary the user reads.
fn reported(cli: &Cli, run: (RunReport, String)) -> Result<(RunStatus, String)> {
    let (report, rendered) = run;
    let output = if cli.json {
        serde_json::to_string_pretty(&report)? + "\n"
    } else {
        rendered
    };
    Ok((report.status(), output))
}

/// Resolves where the user's own game profiles live, unless `--games-dir` said.
fn user_games_dir(override_dir: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = override_dir {
        return Ok(dir);
    }
    dirs::user_games_dir().context("could not determine a config directory for this user")
}

/// Resolves where the journal, backups, and logs live, unless `--state-dir` said.
fn state_paths(override_dir: Option<PathBuf>) -> Result<StatePaths> {
    if let Some(dir) = override_dir {
        return Ok(StatePaths::new(dir));
    }
    let root = dirs::state_dir().context("could not determine a state directory for this user")?;
    Ok(StatePaths::new(root))
}
