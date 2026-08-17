//! Resolve step ids and build the set a command should run.

use std::path::Path;

use anyhow::{anyhow, Context as _, Result};
use gameready_core::improvement::{CoreImprovement, ImprovementId};
use gameready_core::steps::{core_steps, find_core_step, game_steps};

use crate::features::{build_game_step, is_game_step, GameStepBuildError};

/// Selects core steps for an optional `--step` flag.
pub(crate) fn select_steps(step: Option<&str>) -> Result<Vec<Box<dyn CoreImprovement>>> {
    let Some(requested) = step else {
        return Ok(core_steps());
    };
    Ok(vec![find_step_core(requested)?])
}

/// Selects core and per-game steps for the selftest sweep or a named step.
pub(crate) fn select_steps_including_games(
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

/// Resolves the per-game steps, using inert steps when Steam is unavailable.
fn game_steps_or_inert(user_games_dir: &Path) -> Vec<Box<dyn CoreImprovement>> {
    game_steps()
        .into_iter()
        .map(|inert| match build_game_step(&inert.id(), user_games_dir) {
            Ok(step) => step,
            Err(GameStepBuildError::SteamUnavailable | GameStepBuildError::NoGames { .. }) => inert,
        })
        .collect()
}

/// Finds any core or per-game step by id.
pub(crate) fn find_step(requested: &str) -> Result<Box<dyn CoreImprovement>> {
    find_step_in(requested, core_steps().into_iter().chain(game_steps()))
}

/// Finds a core step by id.
pub(crate) fn find_step_core(requested: &str) -> Result<Box<dyn CoreImprovement>> {
    find_step_in(requested, core_steps())
}

/// Matches an id against a known collection and names every valid option on failure.
fn find_step_in(
    requested: &str,
    known: impl IntoIterator<Item = Box<dyn CoreImprovement>>,
) -> Result<Box<dyn CoreImprovement>> {
    let id = ImprovementId::parse(requested)
        .with_context(|| format!("`{requested}` is not a step id"))?;
    find_core_step(&id).ok_or_else(|| {
        let known: Vec<String> = known
            .into_iter()
            .map(|step| step.id().to_string())
            .collect();
        anyhow!(
            "no step named `{requested}`. There is: {}",
            known.join(", ")
        )
    })
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
