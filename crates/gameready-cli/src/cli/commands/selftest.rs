//! `gameready selftest`.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{RunStatus, StepSelftest, selftest};
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::ui::SelftestSummary;

/// Applies each step, verifies it, rolls it back, and verifies it reverted.
///
/// The only way to prove a step that touches kernel state works: containers
/// cannot write `/proc/sys`, and CI cannot repoint a live scheduler.
pub fn run(runner: &dyn CommandRunner, paths: StatePaths) -> Result<(RunStatus, String)> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());
    let mut journal = Journal::open(paths, RunId::generate())?;

    let results = selftest(core_steps(), &cx, runner, &mut journal);
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
