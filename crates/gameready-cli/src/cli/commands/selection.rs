//! Turning a step id into the steps a command runs.

use anyhow::{Context as _, Result, anyhow};
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
    Ok(vec![find_step(requested)?])
}

/// One step by id, or an error naming the ids there are.
///
/// A typo in an id is the most likely way to reach this, and the ids are not
/// something anyone remembers, so the error carries them rather than sending
/// the user off to another command to find out.
pub fn find_step(requested: &str) -> Result<Box<dyn CoreImprovement>> {
    let id = ImprovementId::parse(requested)
        .with_context(|| format!("`{requested}` is not a step id"))?;

    find_core_step(&id).ok_or_else(|| {
        let known: Vec<String> = core_steps()
            .iter()
            .map(|step| step.id().to_string())
            .collect();
        anyhow!(
            "no step named `{requested}`. There is: {}",
            known.join(", ")
        )
    })
}

#[cfg(test)]
#[path = "selection_test.rs"]
mod selection_test;
