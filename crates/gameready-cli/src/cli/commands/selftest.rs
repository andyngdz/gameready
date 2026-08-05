//! `gameready selftest`.

use std::fmt::Write as _;

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::{ApplyCx, CoreCx};
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::RunStatus;
use gameready_core::steps::core_steps;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;

/// Applies each step, verifies it, rolls it back, and verifies it reverted.
///
/// The only way to prove a step that touches kernel state works: containers
/// cannot write `/proc/sys`, and CI cannot repoint a live scheduler.
pub fn run(runner: &dyn CommandRunner, paths: StatePaths) -> Result<(RunStatus, String)> {
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let cx = CoreCx::new(&facts, runner);
    let mut journal = Journal::open(paths, RunId::generate())?;

    let mut out = String::new();
    writeln!(out, "\nSelftest")?;
    let mut all_passed = true;

    for step in core_steps() {
        let mut apply_cx = ApplyCx::new(cx, step.id(), runner, &mut journal);
        let applied = step.apply(&mut apply_cx);
        let recorded = apply_cx.recorded().to_vec();

        let after_apply = applied.is_ok() && step.verify(&cx).is_ok_and(|v| v.passed());
        let undone = step.rollback(&recorded, &mut apply_cx).is_ok();
        // Verification must now *fail*: the change is supposed to be gone.
        let after_rollback = !step.verify(&cx).is_ok_and(|v| v.passed());

        let passed = after_apply && undone && after_rollback;
        all_passed &= passed;

        writeln!(
            out,
            "  {}  {}   apply={after_apply} rollback={undone} reverted={after_rollback}",
            if passed { "ok" } else { "!!" },
            step.id(),
        )?;
    }

    let status = if all_passed {
        RunStatus::Clean
    } else {
        RunStatus::StepFailed
    };
    Ok((status, out))
}

#[cfg(test)]
#[path = "selftest_test.rs"]
mod selftest_test;
