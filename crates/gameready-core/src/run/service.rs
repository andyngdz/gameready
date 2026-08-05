//! Running a set of improvements.

use std::time::Instant;

use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::{
    CoreCx, CoreImprovement, ImprovementId, Outcome, Probe, RollbackStatus, SkipReason,
};
use crate::journal::{Change, Journal, JournalEvent};
use crate::pkg::PackageManager;
use crate::run::domain::{Mode, RunEvent, RunReport, StepReport};
use crate::run::errors::RunError;
use crate::run::use_cases::apply_step::apply_and_verify;
use crate::run::use_cases::resolve::resolve_dependencies;

/// Probes, resolves dependencies, installs prerequisites, applies, and verifies
/// a set of steps.
pub fn execute(
    steps: Vec<Box<dyn CoreImprovement>>,
    facts: &SystemFacts,
    runner: &dyn CommandRunner,
    journal: &mut Journal,
    mode: Mode,
    pkg_manager: Option<&dyn PackageManager>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<RunReport, RunError> {
    let started = Instant::now();
    let cx = CoreCx::new(facts, runner);

    let mut reports = Vec::with_capacity(steps.len());
    let mut pending = Vec::new();

    for step in steps {
        on_event(RunEvent::Probing { step: step.id() });
        match probe_outcome(step.as_ref(), &cx, mode) {
            Settled::Now(outcome) => reports.push(step_report(step.as_ref(), outcome)),
            Settled::Apply => pending.push(step),
        }
    }

    on_event(RunEvent::Planned {
        applicable: pending.len(),
        skipped: reports.len(),
    });

    let installed_deps = resolve_and_install(
        &mut pending,
        &mut reports,
        &cx,
        journal,
        mode,
        pkg_manager,
        on_event,
    )?;

    if mode.mutates() {
        apply_all(pending, &cx, runner, journal, &mut reports, on_event)?;
    }

    Ok(RunReport {
        run: journal.run(),
        mode,
        steps: reports,
        installed_dependencies: installed_deps,
        took: started.elapsed(),
    })
}

fn resolve_and_install(
    pending: &mut Vec<Box<dyn CoreImprovement>>,
    reports: &mut Vec<StepReport>,
    cx: &CoreCx<'_>,
    journal: &mut Journal,
    mode: Mode,
    pkg_manager: Option<&dyn PackageManager>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<Vec<String>, RunError> {
    let pm = match pkg_manager {
        Some(pm) if !pending.is_empty() => pm,
        _ => return Ok(Vec::new()),
    };

    let preflight = resolve_dependencies(pending, cx.facts, cx.runner, pm);
    demote_blocked(pending, reports, &preflight);

    on_event(RunEvent::DependenciesResolved {
        report: preflight.clone(),
    });

    if !mode.mutates() || !preflight.needs_install() {
        return Ok(Vec::new());
    }

    let packages = preflight.packages_to_install(cx.facts.distro.package_manager());
    on_event(RunEvent::InstallingDependencies {
        count: packages.len(),
    });

    let outcome = pm.install(cx.runner, &packages)?;

    if outcome.changed_anything() {
        journal.append(JournalEvent::Changed {
            step: ImprovementId::from_static("preflight.dependencies"),
            change: Change::PackagesInstalled {
                manager: pm.kind().binary().to_owned(),
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

fn demote_blocked(
    pending: &mut Vec<Box<dyn CoreImprovement>>,
    reports: &mut Vec<StepReport>,
    preflight: &crate::run::domain::PreflightReport,
) {
    let blocked = preflight.blocked_steps();
    if blocked.is_empty() {
        return;
    }
    pending.retain(|step| {
        if blocked.contains(&step.id()) {
            reports.push(step_report(
                step.as_ref(),
                Outcome::NotApplicable {
                    reason: "a required dependency is unavailable on this system".to_owned(),
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
    runner: &dyn CommandRunner,
    journal: &mut Journal,
    reports: &mut Vec<StepReport>,
    on_event: &mut dyn FnMut(RunEvent),
) -> Result<(), RunError> {
    for step in pending {
        on_event(RunEvent::Applying {
            step: step.id(),
            name: step.name().to_owned(),
        });

        journal.append(JournalEvent::StepBegin { step: step.id() })?;
        let outcome = apply_and_verify(step.as_ref(), cx, runner, journal);

        journal.append(JournalEvent::StepEnd {
            step: step.id(),
            outcome: outcome.label().to_owned(),
        })?;
        on_event(RunEvent::Finished {
            step: step.id(),
            label: outcome.label().to_owned(),
        });

        reports.push(step_report(step.as_ref(), outcome));
    }
    Ok(())
}

enum Settled {
    Now(Outcome),
    Apply,
}

fn probe_outcome(step: &dyn CoreImprovement, cx: &CoreCx<'_>, mode: Mode) -> Settled {
    match step.probe(cx) {
        Ok(Probe::Applicable) if mode.mutates() => Settled::Apply,
        Ok(Probe::Applicable) => Settled::Now(Outcome::Skipped {
            reason: SkipReason::DryRun,
        }),
        Ok(Probe::AlreadyApplied { evidence }) => {
            Settled::Now(Outcome::AlreadyApplied { evidence })
        }
        Ok(Probe::NotApplicable { reason }) => Settled::Now(Outcome::NotApplicable { reason }),
        Ok(Probe::Conflict { with, detail: _ }) => Settled::Now(Outcome::Skipped {
            reason: SkipReason::Conflict { with },
        }),
        Ok(Probe::Unknown { reason }) => Settled::Now(Outcome::NotApplicable { reason }),
        Err(error) => Settled::Now(Outcome::Failed {
            error: error.to_string(),
            rolled_back: RollbackStatus::NotAttempted,
        }),
    }
}

fn step_report(step: &dyn CoreImprovement, outcome: Outcome) -> StepReport {
    StepReport {
        step: step.id(),
        name: step.name().to_owned(),
        outcome,
    }
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
