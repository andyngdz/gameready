//! Quitting Steam and writing the settings it holds.

use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::{CoreCx, CoreImprovement};
use crate::infra::steam::config::SteamConfigs;
use crate::infra::steam::process::{is_running, shutdown, start};
use crate::journal::Journal;
use crate::run::{InstallConsent, Mode, RunError, RunReport, execute};
use crate::steps::{CompatTarget, LaunchTarget, SteamLaunchOptions, SteamProton};

/// What is to be written into them.
#[derive(Debug, Clone, Default)]
pub struct SteamSettings {
    pub launch: Vec<LaunchTarget>,
    pub proton: Vec<CompatTarget>,
}

/// Quits Steam, then writes every setting the user agreed to.
///
/// Steam is stopped first because it holds its config in memory and rewrites
/// both files when it exits, so a write made while it runs is discarded without
/// a word. It is stopped once for both files rather than once each: a user who
/// agreed to close Steam agreed to close it, not to close it twice.
///
/// Each step backs up the whole file it touches before writing, so
/// `gameready rollback` puts back exactly what Steam had, including anything
/// the user had set themselves.
///
/// The paths are passed in rather than located here so this can be exercised
/// against a fixture. Locating them needs a real Steam installation, and a test
/// that needs one is a test that only runs on some machines.
pub fn write_steam_settings(
    runner: &dyn CommandRunner,
    facts: &SystemFacts,
    journal: &mut Journal,
    configs: SteamConfigs,
    settings: SteamSettings,
) -> Result<RunReport, RunError> {
    let was_running = is_running(runner);
    if was_running {
        shutdown(runner)?;
    }

    let steps: Vec<Box<dyn CoreImprovement>> = vec![
        Box::new(SteamLaunchOptions::new(configs.local, settings.launch)),
        Box::new(SteamProton::new(configs.install, settings.proton)),
    ];
    // Declined rather than granted because this path has no package tooling and
    // installs nothing: writing config files is the whole job.
    let report = execute(
        steps,
        &CoreCx::new(facts, runner),
        journal,
        Mode::Apply,
        InstallConsent::Declined,
        &mut |_| {},
    )?;

    if was_running {
        start(runner);
    }
    Ok(report)
}

#[cfg(test)]
#[path = "write_settings_test.rs"]
mod write_settings_test;
