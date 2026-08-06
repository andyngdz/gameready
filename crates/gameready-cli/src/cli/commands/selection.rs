//! Turning a `--step` flag into the steps a command runs.

use anyhow::{Context as _, Result};
use gameready_core::improvement::{CoreImprovement, ImprovementId};
use gameready_core::steps::{core_steps, find_core_step};

/// The steps a `--step` flag selects: the one named, or all of them.
///
/// Shared by `apply` and `selftest` so both accept the same ids and fail the
/// same way on an unknown one, rather than one command silently ignoring the
/// flag.
pub fn select_steps(step: Option<&str>) -> Result<Vec<Box<dyn CoreImprovement>>> {
    let Some(requested) = step else {
        return Ok(core_steps());
    };
    let id = ImprovementId::parse(requested)
        .with_context(|| format!("`{requested}` is not a step id"))?;
    let selected = find_core_step(&id).with_context(|| format!("no step named `{requested}`"))?;
    Ok(vec![selected])
}

#[cfg(test)]
#[path = "selection_test.rs"]
mod selection_test;
