//! The catalog of built-in improvements.

use crate::improvement::{CoreImprovement, ImprovementId};
use crate::steps::use_cases::MaxMapCount;

/// Every system-wide improvement gameready ships, in the order they apply.
///
/// Order matters where one step's effect changes what another probes, so this
/// is a list rather than a set. Steps that genuinely depend on each other say
/// so through `requires()` as well; this ordering is the tie-breaker for the
/// ones that merely read better in a particular sequence.
#[must_use]
pub fn core_steps() -> Vec<Box<dyn CoreImprovement>> {
    vec![Box::new(MaxMapCount)]
}

/// Finds one step by id, for `apply --step` and `explain`.
#[must_use]
pub fn find_core_step(id: &ImprovementId) -> Option<Box<dyn CoreImprovement>> {
    core_steps().into_iter().find(|step| &step.id() == id)
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
