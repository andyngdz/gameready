//! `gameready explain`.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::facts;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;

use crate::cli::commands::constants::CANNOT_READ_SYSTEM;
use crate::cli::commands::selection::find_step;
use crate::cli::ui::{StepExplanation, StepIndex};

/// Says what one step does, or lists the steps there are to ask about.
///
/// Reads the machine so the answer is about this machine rather than about
/// gameready in general: "what would this do to me" is the question someone has
/// before they agree to it.
pub fn run(runner: &dyn CommandRunner, step: Option<&str>) -> Result<String> {
    let Some(requested) = step else {
        return Ok(StepIndex::all().to_string());
    };

    let selected = find_step(requested)?;
    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    Ok(StepExplanation::of(selected.as_ref(), &cx).to_string())
}

#[cfg(test)]
#[path = "explain_test.rs"]
mod explain_test;
