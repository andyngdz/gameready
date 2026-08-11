//! Reading the machine into a [`Snapshot`], changing nothing.

use std::path::Path;

use gameready_core::doctor::StepFinding;
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::games::AppId;
use gameready_core::improvement::{CoreCx, CoreImprovement, ProbeStatus};
use gameready_core::infra::pkg;
use gameready_core::infra::steam::{
    configs_under, discover_setups, installed_compat_tools, locate_steam_dir,
};
use gameready_core::run::{compat_targets_for, targets_for};
use gameready_core::steps::{core_steps, SteamLaunchOptions, SteamProton};

use crate::tray::{Row, RowAction, Snapshot, PROTON_GE_ID};

/// Probes every core tuning and describes the state of each.
///
/// Slow by design: several steps shell out to `systemctl`, read `/sys`, or ask
/// the package manager. Call it from a worker, never from the thread serving
/// the menu.
#[must_use]
pub fn sweep(runner: &dyn CommandRunner) -> Snapshot {
    let facts = match facts::probe(runner) {
        Ok(facts) => facts,
        Err(error) => {
            return Snapshot::Unreadable {
                reason: error.to_string(),
            }
        }
    };

    // The package tooling is what lets a step answer "is this in your
    // repositories" rather than "I could not tell".
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    let rows = core_steps()
        .iter()
        .map(|step| {
            let finding = StepFinding::of(step.as_ref(), &cx);
            // The one clickable row is Proton-GE with an update pending; every
            // other row, including every game row, stays read-only.
            let action = (step.id() == PROTON_GE_ID
                && finding.status() == ProbeStatus::UpdateAvailable)
                .then_some(RowAction::UpdateProtonGe);
            Row::new(step.bar_name(), &finding, action)
        })
        .collect();
    Snapshot::Ready { rows }
}

/// What gameready set for one game, as its own rows.
///
/// Empty when Steam cannot be located, when the running game has no profile,
/// or when the machine could not be read. A submenu with nothing under it is
/// not drawn, which is the honest answer to "there is nothing to say here".
#[must_use]
pub fn sweep_game(runner: &dyn CommandRunner, app_id: AppId, user_games: &Path) -> Vec<Row> {
    let (Ok(facts), Ok(steam)) = (facts::probe(runner), locate_steam_dir()) else {
        return Vec::new();
    };
    let Ok(configs) = configs_under(&steam) else {
        return Vec::new();
    };
    let Some(setup) = discover_setups(user_games)
        .into_iter()
        .find(|setup| setup.game.app_id == app_id)
    else {
        return Vec::new();
    };

    // The same targets `init` would write, so a row says what a run would put
    // there rather than what the profile happens to contain.
    let running = std::slice::from_ref(&setup);
    let steps: Vec<Box<dyn CoreImprovement>> = vec![
        Box::new(SteamLaunchOptions::new(configs.local, targets_for(running))),
        Box::new(SteamProton::new(
            configs.install,
            compat_targets_for(running, &installed_compat_tools(&steam)),
        )),
    ];

    let cx = CoreCx::new(&facts, runner);
    steps
        .iter()
        .map(|step| Row::new(step.bar_name(), &StepFinding::of(step.as_ref(), &cx), None))
        .collect()
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
