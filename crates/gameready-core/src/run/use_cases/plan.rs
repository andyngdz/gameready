//! Working out what a run would do, without doing any of it.
//!
//! Every call here reads: probes ask the system questions, the resolver asks
//! the package manager questions, and neither writes. That is what makes it
//! safe to run before the user has been asked anything.

use crate::improvement::{CoreCx, CoreImprovement, ImprovementId, Outcome, PlannedAction};
use crate::run::domain::{
    Deferred, PlannedInstall, PreflightReport, RunEvent, RunPlan, StepReport,
};
use crate::run::use_cases::probe::{probe_all, Probed};
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
    let Probed {
        mut settled,
        mut pending,
        mut deferred,
    } = probe_all(steps, cx, on_event);

    // Held-open steps are resolved alongside pending ones on purpose. A step
    // promoted halfway through the run must not fetch a package the user never
    // saw, so what it needs goes on the same screen as everybody else's.
    let preflight = resolve(&considered(&pending, &deferred), cx);

    demote_blocked(&mut pending, &mut deferred, &mut settled, &preflight);

    on_event(RunEvent::DependenciesResolved {
        report: preflight.clone(),
    });

    let StepInstalls {
        installs: step_installs,
        present: step_present,
    } = self_installs(&considered(&pending, &deferred), cx);

    RunPlan {
        settled,
        pending,
        deferred,
        preflight,
        step_installs,
        step_present,
        started,
    }
}

/// Every step the run may still apply, in the order a user would meet them.
fn considered<'a>(
    pending: &'a [Box<dyn CoreImprovement>],
    deferred: &'a [Deferred],
) -> Vec<&'a dyn CoreImprovement> {
    pending
        .iter()
        .map(AsRef::as_ref)
        .chain(deferred.iter().map(|held| held.step.as_ref()))
        .collect()
}

/// What the steps say about packages, from their own plans.
struct StepInstalls {
    installs: Vec<(ImprovementId, PlannedInstall)>,
    present: Vec<String>,
}

/// Packages the steps would install themselves.
///
/// Read off each step's own plan rather than from `dependencies()`, because a
/// step whose whole job is installing something declares it as the work it
/// does, not as a prerequisite. Both routes end with software on the machine,
/// so both have to reach the screen that asks.
///
/// A step whose `plan` errors contributes nothing here. It will fail on its own
/// terms during apply, and guessing what it might have installed would put a
/// package name in front of the user that no step ever asked for.
fn self_installs(steps: &[&dyn CoreImprovement], cx: &CoreCx<'_>) -> StepInstalls {
    let mut installs = Vec::new();
    let mut present = Vec::new();

    for step in steps {
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

/// What the steps need, or an empty report when there is nothing to ask about.
///
/// A run with no package tooling cannot answer the question, and an empty
/// report says "nothing to install" rather than pretending a check happened.
fn resolve(steps: &[&dyn CoreImprovement], cx: &CoreCx<'_>) -> PreflightReport {
    match cx.packages {
        Some(packages) if !steps.is_empty() => {
            resolve_dependencies(steps, cx.facts, cx.runner, packages)
        }
        _ => PreflightReport {
            dependencies: Vec::new(),
            total_install_bytes: 0,
        },
    }
}

/// Moves steps whose dependency this distro does not carry out of the run.
///
/// Separate from a declined install: nothing the user could say would make the
/// package appear, so this is `NotApplicable` rather than a skip. A held-open
/// step goes the same way, because a second probe cannot conjure a package the
/// repositories do not have.
fn demote_blocked(
    pending: &mut Vec<Box<dyn CoreImprovement>>,
    deferred: &mut Vec<Deferred>,
    settled: &mut Vec<StepReport>,
    preflight: &PreflightReport,
) {
    let blocked = preflight.blocked_steps();
    if blocked.is_empty() {
        return;
    }
    pending.retain(|step| keep_unless_blocked(step.as_ref(), &blocked, settled));
    deferred.retain(|held| keep_unless_blocked(held.step.as_ref(), &blocked, settled));
}

fn keep_unless_blocked(
    step: &dyn CoreImprovement,
    blocked: &[ImprovementId],
    settled: &mut Vec<StepReport>,
) -> bool {
    if !blocked.contains(&step.id()) {
        return true;
    }
    settled.push(StepReport::for_step(
        step,
        Outcome::NotApplicable {
            reason: "a required dependency is unavailable on this system".to_owned(),
        },
    ));
    false
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
