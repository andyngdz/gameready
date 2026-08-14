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
/// `selftest` is the one command that exists to prove every step works, so it
/// runs every step. Leaving two of thirteen out of the sweep would make "all
/// passed" mean something narrower than it reads.
///
/// A missing Steam is handled differently depending on how the step was
/// reached. In the sweep it is a skip, the same as a missing GPU: a true fact
/// about the machine, not a failure. Named outright it is an error, because
/// somebody asked for that step specifically and a skip would answer a
/// question they did not ask.
pub fn select_steps_including_games(
    step: Option<&str>,
    user_games_dir: &Path,
) -> Result<Vec<Box<dyn CoreImprovement>>> {
    let Some(requested) = step else {
        let mut every = core_steps();
        every.extend(game_steps_or_inert(user_games_dir));
        return Ok(every);
    };

    let id = ImprovementId::parse(requested)
        .with_context(|| format!("`{requested}` is not a step id"))?;
    if is_game_step(&id) {
        return Ok(vec![build_game_step(&id, user_games_dir)?]);
    }
    Ok(vec![find_step(requested)?])
}

/// The per-game steps built against the real Steam, or inert when there is no
/// Steam to build them against.
///
/// The inert ones carry no config path and no targets, so they probe as not
/// applicable and the summary says which machine fact stopped them. That is
/// the same answer a container gets for the shader cache step.
fn game_steps_or_inert(user_games_dir: &Path) -> Vec<Box<dyn CoreImprovement>> {
    game_steps()
        .into_iter()
        .map(|inert| build_game_step(&inert.id(), user_games_dir).unwrap_or(inert))
        .collect()
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
