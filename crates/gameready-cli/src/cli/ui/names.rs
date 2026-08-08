//! What each step is called on a screen, as opposed to in the journal.

use std::collections::HashMap;

use gameready_core::improvement::ImprovementId;
use gameready_core::steps::{core_steps, game_steps};

/// Every step's short name, keyed by id.
///
/// Built once by whoever is about to render a list of results. Events carry the
/// id a step was recorded under, and a screen that printed that would be asking
/// the reader to recognise `core.sysctl.max-map-count` as the thing they agreed
/// to a moment ago under a different name.
#[must_use]
pub fn short_names() -> HashMap<ImprovementId, String> {
    core_steps()
        .iter()
        .chain(game_steps().iter())
        .map(|step| (step.id(), step.short_name().to_owned()))
        .collect()
}

#[cfg(test)]
#[path = "names_test.rs"]
mod names_test;
