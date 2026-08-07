//! `gameready selftest`.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{selftest, RunStatus, StepSelftest};

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::commands::selection::select_steps;
use crate::cli::escalation::Escalation;
use crate::cli::ui::SelftestSummary;

/// Applies each step, verifies it, rolls it back, and verifies it reverted.
///
/// The only way to prove a step that touches kernel state works: containers
/// cannot write `/proc/sys`, and CI cannot repoint a live scheduler. `step`
/// limits the run to one id, the same as `apply --step`.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    step: Option<&str>,
    escalation: Escalation<'_>,
) -> Result<(RunStatus, String)> {
    let selected = select_steps(step)?;
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    // Asked here rather than at the top, so an unknown step id and an unreadable
    // system both answer without a password.
    escalation.ask()?;

    let mut journal = Journal::open(paths, RunId::generate())?;

    let results = selftest(selected, &cx, runner, &mut journal);
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
