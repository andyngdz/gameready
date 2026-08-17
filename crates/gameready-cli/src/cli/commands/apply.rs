//! `gameready apply`.

use anyhow::{Context as _, Result};
use gameready_core::facts;

use crate::cli::commands::constants::{CANNOT_OPEN_JOURNAL, CANNOT_READ_SYSTEM};
use crate::features::select_steps;
use gameready_core::exec::CommandRunner;
use gameready_core::improvement::CoreCx;
use gameready_core::infra::pkg;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{apply_plan, plan_run, InstallConsent, Mode, RunPlan, RunReport};

use crate::cli::escalation::Escalation;
use crate::cli::ui::{self, Picker};

/// Probes, applies, and verifies the selected steps.
///
/// The install question comes before the journal is opened and before the first
/// change, for the same reason `init` asks everything up front: an answer given
/// after the fact is not really an answer.
pub fn run(
    runner: &dyn CommandRunner,
    paths: StatePaths,
    step: Option<&str>,
    mode: Mode,
    picker: Picker,
    escalation: Escalation<'_>,
) -> Result<(RunReport, String)> {
    let selected = select_steps(step)?;

    let facts = facts::probe(runner).context(CANNOT_READ_SYSTEM)?;
    let packages = pkg::for_kind(facts.distro.package_manager());
    let family = facts.distro.package_manager();
    let cx = CoreCx::new(&facts, runner).with_packages(packages.as_ref());

    let mut progress = ui::ProgressView::new();
    let plan = plan_run(selected, &cx, &mut |event| progress.on_event(event));
    drop(progress);

    let (consent, takeovers, listed) = Approvals::ask(&plan, family, picker, mode)?;

    // Nothing above this line changed anything. Nothing below it asks, and it
    // only asks when a step in the run reaches outside the user's own files.
    if plan.needs_root() {
        escalation.ask()?;
    }

    let mut journal =
        Journal::open(paths.clone(), RunId::generate()).context(CANNOT_OPEN_JOURNAL)?;
    let mut progress = ui::ProgressView::sweeping(mode, plan.to_apply());
    let report = apply_plan(
        plan,
        &cx,
        &mut journal,
        mode,
        consent,
        &takeovers,
        &mut |event| {
            progress.on_event(event);
        },
    )
    .context("the run could not complete")?;
    drop(progress);

    let rendered = listed + &ui::Summary::new(&report, &paths.journal()).to_string();
    Ok((report, rendered))
}

/// Every answer a run needs before it may change anything.
///
/// The install consent, which contested steps the user agreed to take over,
/// and (for a dry run) the list of what would have been installed. Nothing
/// here changes the machine, and nothing after here asks.
struct Approvals;

impl Approvals {
    fn ask(
        plan: &RunPlan,
        family: gameready_core::facts::PackageManagerKind,
        picker: Picker,
        mode: Mode,
    ) -> Result<(
        InstallConsent,
        Vec<gameready_core::improvement::ImprovementId>,
        String,
    )> {
        let consent = ui::consent_to_install(plan, family, picker, mode)?;
        // A contested step is a decision, not a skip: the user gets the same
        // takeover question `init` would have asked, so `apply --step` on a
        // scheduler somebody else runs does not silently stand down. A run
        // that cannot ask keeps the safe answer, and a dry run changes nothing.
        let takeovers = match (picker, mode.mutates()) {
            (Picker::Ask, true) => ui::ask_takeovers(&plan.contested)?,
            _ => Vec::new(),
        };
        // A real run installs a moment later and the summary reports what
        // landed. A dry run has only this list to say what it would have done.
        let listed = if mode.mutates() {
            String::new()
        } else {
            ui::InstallList::new(plan, family).to_string()
        };
        Ok((consent, takeovers, listed))
    }
}

#[cfg(test)]
#[path = "apply_test.rs"]
mod apply_test;
