//! `gameready init`.

use std::path::Path;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::infra::pkg;
use gameready_core::infra::steam::{discover_setups, locate_local_config, write_launch_options};
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{Mode, RunReport, execute};
use gameready_core::steam::Overlay;
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::{CANNOT_OPEN_JOURNAL, CANNOT_READ_SYSTEM};
use crate::cli::ui::{self, LaunchChoice, Picker};

/// What the flags asked for, before any question is put to the user.
pub struct InitRequest<'a> {
    pub games_dir: &'a Path,
    pub mode: Mode,
    pub picker: Picker,
    /// `Some` only when a flag set it; otherwise the run asks.
    pub overlay: Option<Overlay>,
}

/// Asks everything, then does everything.
///
/// The order is the contract. Every question the run has is answered before the
/// first change, including the password prompt, so a user is never asked to
/// decide something once the alternative has already been taken away from them.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    request: &InitRequest<'_>,
    authorize: &dyn Fn() -> Result<()>,
) -> Result<(RunReport, String)> {
    let setups = discover_setups(request.games_dir);
    let answers = ui::ask_everything(&setups, request.picker, request.overlay, request.mode)?;

    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let plan = ui::InitPlan::new(&setups, &answers, request.mode).to_string();

    // Nothing above this line changed anything. Nothing below it asks.
    authorize()?;

    let packages = pkg::for_kind(facts.distro.package_manager());
    let mut journal =
        Journal::open(paths.clone(), RunId::generate()).context(CANNOT_OPEN_JOURNAL)?;
    let mut progress = ui::ProgressView::new();
    let report = execute(
        core_steps(),
        &facts,
        runner,
        &mut journal,
        request.mode,
        Some(packages.as_ref()),
        &mut |event| progress.on_event(event),
    )?;
    drop(progress);
    let mut out = plan;
    out.push_str(&ui::Summary::new(&report, &paths.journal()).to_string());

    if !answers.targets.is_empty() {
        let launch_text = match answers.launch {
            LaunchChoice::ShowForCopying => {
                ui::LaunchInstructions::new(&answers.selected).to_string()
            }
            LaunchChoice::CloseSteamAndWrite => {
                let config =
                    locate_local_config().context("could not find your Steam user config")?;
                let lr = write_launch_options(
                    runner,
                    &facts,
                    &mut journal,
                    config,
                    answers.targets.clone(),
                )?;
                ui::LaunchReport::new(&lr).to_string()
            }
        };
        out.push_str(&launch_text);
    }
    Ok((report, out))
}

#[cfg(test)]
#[path = "init_test.rs"]
mod init_test;
