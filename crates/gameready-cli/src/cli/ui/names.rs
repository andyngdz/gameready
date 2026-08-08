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

/// How wide a column has to be to hold every one of these names.
///
/// Every screen that lists results is a table, and this is what sets the edge
/// the evidence starts at. A screen measures the names it is about to print,
/// except where the catalog is the better answer: see [`name_column`].
#[must_use]
pub fn widest<'a>(names: impl Iterator<Item = &'a str>) -> usize {
    names.map(console::measure_text_width).max().unwrap_or(0)
}

/// How wide the name column in a list of steps is.
///
/// Taken from the widest name in the catalog rather than from whichever steps
/// this run happens to report on. Two runs of the same machine that settle
/// different steps would otherwise line their evidence up at different columns,
/// and a user comparing them would read that as a difference in the result.
#[must_use]
pub fn name_column(names: &HashMap<ImprovementId, String>) -> usize {
    widest(names.values().map(String::as_str))
}

#[cfg(test)]
#[path = "names_test.rs"]
mod names_test;
