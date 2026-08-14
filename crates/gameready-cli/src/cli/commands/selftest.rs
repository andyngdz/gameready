//! `gameready selftest`.

use std::path::Path;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::infra::steam::{is_running, shutdown, start};
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{selftest, RunStatus, StepSelftest};

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::commands::game_steps::is_game_step;
use crate::cli::commands::selection::select_steps_including_games;
use crate::cli::escalation::Escalation;
use crate::cli::ui::SelftestSummary;

/// Applies each step, verifies it, rolls it back, and verifies it reverted.
///
/// The only way to prove a step that touches kernel state works: an
/// unprivileged container cannot write `/proc/sys` at all, and one that can is
/// sharing the host's kernel. `step` limits the run to one id, the same as
/// `apply --step`, and is the only way to reach the per-game steps.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    step: Option<&str>,
    user_games_dir: &Path,
    escalation: Escalation<'_>,
) -> Result<(RunStatus, String)> {
    let selected = select_steps_including_games(step, user_games_dir)?;
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    // Asked here rather than at the top, so an unknown step id and an unreadable
    // system both answer without a password.
    escalation.ask()?;

    let mut journal = Journal::open(paths, RunId::generate())?;

    // Steam holds both config files in memory and writes them out when it
    // exits, so a selftest of a Steam step against a running Steam would have
    // its apply, its rollback, or both thrown away without a word.
    let touches_steam = selected.iter().any(|step| is_game_step(&step.id()));
    let steam_was_running = touches_steam && is_running(runner);
    if steam_was_running {
        shutdown(runner)?;
    }

    let results = selftest(selected, &cx, runner, &mut journal);

    if steam_was_running {
        start(runner);
    }
    let status = if results.iter().any(StepSelftest::is_failure) {
        RunStatus::StepFailed
    } else {
        RunStatus::Clean
    };

    Ok((status, SelftestSummary::new(&results).to_string()))
}

#[cfg(test)]
#[path = "selftest_test.rs"]
mod selftest_test;
