//! The catalog of built-in improvements.

use crate::improvement::{CoreImprovement, ImprovementId};
use crate::steps::use_cases::{Conflicts, CpuGovernor, GamingTools, MaxMapCount};

/// Every system-wide improvement gameready ships, in the order they apply.
///
/// Order matters where one step's effect changes what another probes, so this
/// is a list rather than a set. Steps that genuinely depend on each other say
/// so through `requires()` as well; this ordering is the tie-breaker for the
/// ones that merely read better in a particular sequence.
#[must_use]
pub fn core_steps() -> Vec<Box<dyn CoreImprovement>> {
    vec![
        // Conflicts first: what it finds explains why gamemode may look like it
        // is doing nothing, and the user should read that before the steps that
        // install gamemode and defer to it.
        Box::new(Conflicts),
        Box::new(MaxMapCount),
        Box::new(GamingTools),
        Box::new(CpuGovernor),
    ]
}

/// Finds one step by id, for `apply --step` and `explain`.
#[must_use]
pub fn find_core_step(id: &ImprovementId) -> Option<Box<dyn CoreImprovement>> {
    core_steps().into_iter().find(|step| &step.id() == id)
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
