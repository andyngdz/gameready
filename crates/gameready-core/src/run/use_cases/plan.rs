//! Working out what a run would do, without doing any of it.
//!
//! Every call here reads: probes ask the system questions, the resolver asks
//! the package manager questions, and neither writes. That is what makes it
//! safe to run before the user has been asked anything.

use crate::improvement::{
    CoreCx, CoreImprovement, ImprovementId, Outcome, PlannedAction, Probe, RollbackStatus,
    SkipReason,
};
use crate::run::domain::{PlannedInstall, PreflightReport, RunEvent, RunPlan, StepReport};
use crate::run::use_cases::resolve::resolve_dependencies;

/// Probes every step and resolves what the survivors need.
///
/// Returns without touching the system, so the caller can render the whole
/// picture and ask its questions while every answer is still available.
///
/// The run's mode is deliberately not consulted here. A dry run has to reach
/// the same list of missing packages a real run would, or it cannot tell the
/// user what a real run would install.
pub fn plan_run(
    steps: Vec<Box<dyn CoreImprovement>>,
    cx: &CoreCx<'_>,
    on_event: &mut dyn FnMut(RunEvent),
) -> RunPlan {
    let started = std::time::Instant::now();
    let (mut settled, mut pending) = probe_all(steps, cx, on_event);
    let preflight = resolve(&pending, cx);

    demote_blocked(&mut pending, &mut settled, &preflight);

    on_event(RunEvent::DependenciesResolved {
        report: preflight.clone(),
    });

    let StepInstalls {
        installs: step_installs,
        present: step_present,
    } = self_installs(&pending, cx);

    RunPlan {
        settled,
        pending,
        preflight,
        step_installs,
        step_present,
        started,
    }
}

/// What the pending steps say about packages, from their own plans.
struct StepInstalls {
    installs: Vec<(ImprovementId, PlannedInstall)>,
    present: Vec<String>,
}

/// Packages the pending steps would install themselves.
///
/// Read off each step's own plan rather than from `dependencies()`, because a
/// step whose whole job is installing something declares it as the work it
/// does, not as a prerequisite. Both routes end with software on the machine,
/// so both have to reach the screen that asks.
///
/// A step whose `plan` errors contributes nothing here. It will fail on its own
/// terms during apply, and guessing what it might have installed would put a
/// package name in front of the user that no step ever asked for.
fn self_installs(pending: &[Box<dyn CoreImprovement>], cx: &CoreCx<'_>) -> StepInstalls {
    let mut installs = Vec::new();
    let mut present = Vec::new();

    for step in pending {
        let Ok(plan) = step.plan(cx) else { continue };
        for action in &plan.actions {
            let PlannedAction::InstallPackages {
                packages,
                already_present,
            } = action
            else {
                continue;
            };
            installs.extend(packages.iter().map(|package| {
                (
                    step.id(),
                    PlannedInstall {
                        package: package.name.clone(),
                        what: package.what.clone(),
                        why: package.why.clone(),
                        approx_bytes: package.approx_bytes,
                    },
                )
            }));
            present.extend(already_present.iter().cloned());
        }
    }
    StepInstalls { installs, present }
}

/// What the pending steps need, or an empty report when there is nothing to ask
/// about.
///
/// A run with no package tooling cannot answer the question, and an empty
/// report says "nothing to install" rather than pretending a check happened.
fn resolve(pending: &[Box<dyn CoreImprovement>], cx: &CoreCx<'_>) -> PreflightReport {
    match cx.packages {
        Some(packages) if !pending.is_empty() => {
            resolve_dependencies(pending, cx.facts, cx.runner, packages)
        }
        _ => PreflightReport {
            dependencies: Vec::new(),
            total_install_bytes: 0,
        },
    }
}

fn probe_all(
    steps: Vec<Box<dyn CoreImprovement>>,
    cx: &CoreCx<'_>,
    on_event: &mut dyn FnMut(RunEvent),
) -> (Vec<StepReport>, Vec<Box<dyn CoreImprovement>>) {
    let mut settled = Vec::with_capacity(steps.len());
    let mut pending = Vec::new();

    for step in steps {
        on_event(RunEvent::Probing { step: step.id() });
        match probe_outcome(step.as_ref(), cx) {
            Settled::Now(outcome) => {
                on_event(RunEvent::Finished {
                    step: step.id(),
                    name: step.name().to_owned(),
                    kind: outcome.kind(),
                    detail: outcome.detail(),
                });
                settled.push(StepReport::for_step(step.as_ref(), outcome));
            }
            Settled::Apply => pending.push(step),
        }
    }

    on_event(RunEvent::Planned {
        applicable: pending.len(),
        skipped: settled.len(),
    });

    (settled, pending)
}

/// Moves steps whose dependency this distro does not carry out of the pending
/// list.
///
/// Separate from a declined install: nothing the user could say would make the
/// package appear, so this is `NotApplicable` rather than a skip.
fn demote_blocked(
    pending: &mut Vec<Box<dyn CoreImprovement>>,
    settled: &mut Vec<StepReport>,
    preflight: &PreflightReport,
) {
    let blocked = preflight.blocked_steps();
    if blocked.is_empty() {
        return;
    }
    pending.retain(|step| {
        if blocked.contains(&step.id()) {
            settled.push(StepReport::for_step(
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

enum Settled {
    Now(Outcome),
    Apply,
}

fn probe_outcome(step: &dyn CoreImprovement, cx: &CoreCx<'_>) -> Settled {
    match step.probe(cx) {
        Ok(Probe::Applicable) => Settled::Apply,
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

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
