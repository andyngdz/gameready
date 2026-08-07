//! Carrying out a plan that has already been agreed to.

use crate::improvement::{CoreCx, CoreImprovement, ImprovementId, Outcome, SkipReason};
use crate::journal::{Change, Journal, JournalEvent};
use crate::run::domain::{InstallConsent, Mode, RunEvent, RunPlan, RunReport, StepReport};
use crate::run::errors::RunError;
use crate::run::use_cases::apply_step::apply_and_verify;
use crate::run::use_cases::plan::plan_run;

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
        preflight,
        started,
        ..
    } = plan;

    if !mode.mutates() {
        settle_as_dry_run(pending, &mut settled);
        return Ok(report(journal, mode, settled, Vec::new(), started));
    }

    let installed = match consent {
        InstallConsent::Granted if preflight.needs_install() => {
            install(&preflight, cx, journal, on_event)?
        }
        InstallConsent::Granted => Vec::new(),
        InstallConsent::Declined => {
            decline(&mut pending, &mut settled, &waiting_on_install);
            Vec::new()
        }
    };

    apply_all(pending, cx, journal, &mut settled, on_event)?;

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
fn settle_as_dry_run(pending: Vec<Box<dyn CoreImprovement>>, settled: &mut Vec<StepReport>) {
    for step in pending {
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
    settled: &mut Vec<StepReport>,
    waiting: &[ImprovementId],
) {
    pending.retain(|step| {
        if waiting.contains(&step.id()) {
            settled.push(StepReport::for_step(
                step.as_ref(),
                Outcome::Skipped {
                    reason: SkipReason::UserDeclined,
                },
            ));
            false
        } else {
            true
        }
    });
}

fn apply_all(
    pending: Vec<Box<dyn CoreImprovement>>,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    settled: &mut Vec<StepReport>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<(), RunError> {
    for step in pending {
        on_event(RunEvent::Applying {
            step: step.id(),
            name: step.name().to_owned(),
        });

        journal.append(JournalEvent::StepBegin { step: step.id() })?;

        let step_id = step.id();
        let progress: Box<dyn FnMut(&str) + '_> = Box::new(|msg: &str| {
            on_event(RunEvent::StepProgress {
                step: step_id.clone(),
                message: msg.to_owned(),
            });
        });
        let outcome = apply_and_verify(step.as_ref(), cx, cx.runner, journal, Some(progress));

        journal.append(JournalEvent::StepEnd {
            step: step.id(),
            outcome: outcome.label().to_owned(),
        })?;
        on_event(RunEvent::Finished {
            step: step.id(),
            name: step.name().to_owned(),
            kind: outcome.kind(),
            detail: outcome.detail(),
        });

        settled.push(StepReport::for_step(step.as_ref(), outcome));
    }
    Ok(())
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;

#[cfg(test)]
#[path = "service_consent_test.rs"]
mod service_consent_test;
