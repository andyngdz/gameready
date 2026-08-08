//! Carrying out a plan that has already been agreed to.

use crate::improvement::{CoreCx, CoreImprovement, ImprovementId, Outcome, SkipReason};
use crate::journal::{Change, Journal, JournalEvent};
use crate::run::domain::{
    Deferred, InstallConsent, Mode, RunEvent, RunPlan, RunReport, StepReport,
};
use crate::run::errors::RunError;
use crate::run::use_cases::plan::plan_run;
use crate::run::use_cases::sweep::apply_all;

/// Plans a run and carries it out in one call.
///
/// `consent` is required rather than defaulted, so a caller that never asked
/// the user cannot install packages by leaving an argument off.
pub fn execute(
    steps: Vec<Box<dyn CoreImprovement>>,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    mode: Mode,
    consent: InstallConsent,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<RunReport, RunError> {
    let plan = plan_run(steps, cx, on_event);
    apply_plan(plan, cx, journal, mode, consent, on_event)
}

/// Installs what the user agreed to, then runs every step that can still run.
///
/// Nothing here asks anything. By the time a plan reaches this function every
/// decision it needed has already been made.
pub fn apply_plan(
    plan: RunPlan,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    mode: Mode,
    consent: InstallConsent,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<RunReport, RunError> {
    let waiting_on_install = plan.steps_needing_install();
    let RunPlan {
        mut settled,
        mut pending,
        mut deferred,
        preflight,
        started,
        ..
    } = plan;

    if !mode.mutates() {
        settle_as_dry_run(pending, deferred, &mut settled);
        return Ok(report(journal, mode, settled, Vec::new(), started));
    }

    let installed = match consent {
        InstallConsent::Granted if preflight.needs_install() => {
            install(&preflight, cx, journal, on_event)?
        }
        InstallConsent::Granted => Vec::new(),
        InstallConsent::Declined => {
            decline(
                &mut pending,
                &mut deferred,
                &mut settled,
                &waiting_on_install,
            );
            Vec::new()
        }
    };

    apply_all(pending, deferred, cx, journal, &mut settled, on_event)?;

    Ok(report(journal, mode, settled, installed, started))
}

fn report(
    journal: &Journal,
    mode: Mode,
    steps: Vec<StepReport>,
    installed_dependencies: Vec<String>,
    started: std::time::Instant,
) -> RunReport {
    RunReport {
        run: journal.run(),
        mode,
        steps,
        installed_dependencies,
        took: started.elapsed(),
    }
}

/// Records every step a real run would have applied, without applying it.
///
/// Held-open steps are listed too. A dry run cannot know how the second probe
/// would answer, but dropping the step entirely would tell the user a real run
/// has nothing to do about it, which is the one thing that is definitely
/// wrong.
fn settle_as_dry_run(
    pending: Vec<Box<dyn CoreImprovement>>,
    deferred: Vec<Deferred>,
    settled: &mut Vec<StepReport>,
) {
    let held = deferred.into_iter().map(|entry| entry.step);
    for step in pending.into_iter().chain(held) {
        settled.push(StepReport::for_step(
            step.as_ref(),
            Outcome::Skipped {
                reason: SkipReason::DryRun,
            },
        ));
    }
}

/// Installs the missing packages in one transaction and journals what was new.
fn install(
    preflight: &crate::run::domain::PreflightReport,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<Vec<String>, RunError> {
    let Some(packages) = cx.packages else {
        return Ok(Vec::new());
    };

    let names = preflight.packages_to_install(cx.facts.distro.package_manager());
    on_event(RunEvent::InstallingDependencies { count: names.len() });

    let outcome = packages.install(cx.runner, &names)?;

    // Only what was genuinely new goes in the journal: recording a package the
    // machine already had would make rollback offer to remove it.
    if outcome.changed_anything() {
        journal.append(JournalEvent::Changed {
            step: ImprovementId::from_static("preflight.dependencies"),
            change: Change::PackagesInstalled {
                manager: packages.kind().binary().to_owned(),
                requested: outcome.requested.clone(),
                newly_installed: outcome.newly_installed.clone(),
            },
        })?;
    }

    on_event(RunEvent::DependenciesInstalled {
        newly_installed: outcome.newly_installed.clone(),
    });

    Ok(outcome.newly_installed)
}

/// Moves every step that would have installed something out of the pending
/// list.
///
/// Covers both routes: a step whose prerequisite was missing, and a step whose
/// own job is putting a package on the machine. The rest of the run goes ahead,
/// because a user who said no to a package did not say no to the steps that
/// need nothing.
fn decline(
    pending: &mut Vec<Box<dyn CoreImprovement>>,
    deferred: &mut Vec<Deferred>,
    settled: &mut Vec<StepReport>,
    waiting: &[ImprovementId],
) {
    pending.retain(|step| keep_unless_waiting(step.as_ref(), waiting, settled));
    deferred.retain(|held| keep_unless_waiting(held.step.as_ref(), waiting, settled));
}

/// Whether one step survives a declined install.
///
/// A held-open step is filtered by the same list as a pending one. Its
/// packages went onto the same screen, so a no there is a no here.
fn keep_unless_waiting(
    step: &dyn CoreImprovement,
    waiting: &[ImprovementId],
    settled: &mut Vec<StepReport>,
) -> bool {
    if !waiting.contains(&step.id()) {
        return true;
    }
    settled.push(StepReport::for_step(
        step,
        Outcome::Skipped {
            reason: SkipReason::UserDeclined,
        },
    ));
    false
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;

#[cfg(test)]
#[path = "service_consent_test.rs"]
mod service_consent_test;
