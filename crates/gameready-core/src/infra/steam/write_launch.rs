//! Quitting Steam and writing the launch options it holds.

use std::path::PathBuf;

use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::CoreImprovement;
use crate::infra::steam::process::{shutdown, start};
use crate::journal::Journal;
use crate::run::{Mode, RunError, RunReport, execute};
use crate::steps::{LaunchTarget, SteamLaunchOptions};

/// Quits Steam, then sets the launch options of every target.
///
/// Steam is stopped first because it holds its config in memory and rewrites
/// the file when it exits, so a write made while it runs is discarded without a
/// word. The step backs the whole file up before touching it, so
/// `gameready rollback` puts back exactly what Steam had, including anything
/// the user had typed into the box themselves.
///
/// `config` is passed in rather than located here so this can be exercised
/// against a fixture. Locating it needs a real Steam installation, and a test
/// that needs one is a test that only runs on some machines.
pub fn write_launch_options(
    runner: &dyn CommandRunner,
    facts: &SystemFacts,
    journal: &mut Journal,
    config: PathBuf,
    targets: Vec<LaunchTarget>,
) -> Result<RunReport, RunError> {
    shutdown(runner)?;

    let step: Box<dyn CoreImprovement> = Box::new(SteamLaunchOptions::new(config, targets));
    let report = execute(
        vec![step],
        facts,
        runner,
        journal,
        Mode::Apply,
        None,
        &mut |_| {},
    )?;

    start(runner);
    Ok(report)
}

#[cfg(test)]
#[path = "write_launch_test.rs"]
mod write_launch_test;
