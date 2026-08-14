//! Turning a step id into the steps a command runs.

use std::path::Path;

use anyhow::{anyhow, Context as _, Result};
use gameready_core::improvement::{CoreImprovement, ImprovementId};
use gameready_core::steps::{core_steps, find_core_step, game_steps};

use crate::cli::commands::game_steps::{build_game_step, is_game_step};

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

/// The same, for a caller that can also run the per-game steps.
///
/// The per-game steps are not in the no-`--step` list on purpose. Writing
/// Steam's config means quitting Steam, and a bare `gameready selftest` closing
/// a running game client is not something a user asked for by typing five
/// words. Naming one is asking for it.
pub fn select_steps_including_games(
    step: Option<&str>,
    user_games_dir: &Path,
) -> Result<Vec<Box<dyn CoreImprovement>>> {
    let Some(requested) = step else {
        return Ok(core_steps());
    };

    let id = ImprovementId::parse(requested)
        .with_context(|| format!("`{requested}` is not a step id"))?;
    if is_game_step(&id) {
        return Ok(vec![build_game_step(&id, user_games_dir)?]);
    }
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
        // The per-game ids are listed too. They are real ids a user can read off
        // `explain`, and leaving them out of the error makes a correct id look
        // like a typo.
        let known: Vec<String> = core_steps()
            .iter()
            .chain(game_steps().iter())
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
