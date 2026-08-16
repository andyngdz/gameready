//! `gameready selftest`.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::{CoreCx, CoreImprovement};
use gameready_core::infra::pkg;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{selftest, RunStatus, StepSelftest};

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::escalation::Escalation;
use crate::cli::ui::SelftestSummary;

/// Applies each step, verifies it, rolls it back, and verifies it reverted.
///
/// The only way to prove a step that touches kernel state works: an
/// unprivileged container cannot write `/proc/sys` at all, and one that can is
/// sharing the host's kernel. `step` limits the run to one id, the same as
/// `apply --step`.
///
/// Every step runs, the two per-game ones included, but Steam itself is left
/// alone: a selftest is a frequent dev and CI tool, and closing a running game
/// client every sweep would be a disruption out of proportion to what it
/// proves. The per-game steps still apply and roll back against the real
/// config files, and Steam only rewrites them when it exits, so a running
/// Steam does not interfere with the test.
///
/// `selected` is resolved by the caller rather than here, because resolving a
/// per-game step means reading the real Steam installation. Taking the list as
/// an argument keeps this a function of its inputs, so a test of it is a test
/// of it rather than of whichever machine ran it.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    selected: Vec<Box<dyn CoreImprovement>>,
    escalation: Escalation<'_>,
) -> Result<(RunStatus, String)> {
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
