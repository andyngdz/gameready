//! What a run worked out about itself before changing anything.

use std::time::Instant;

use crate::facts::PackageManagerKind;
use crate::improvement::{CoreImprovement, ImprovementId};
use crate::run::domain::preflight::PreflightReport;
use crate::run::domain::report::StepReport;

/// One package a run would put on the machine, and who wants it there.
///
/// Packages arrive by two routes. A step can declare a prerequisite through
/// `dependencies()`, which the run installs before any step applies; or a step
/// whose whole job is installing something can do it itself in `apply`. Both
/// end with software on a machine that a rollback will not remove, so both
/// belong on one screen rather than one being asked about and the other not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedInstall {
    /// The package name as this distro's tooling spells it.
    pub package: String,

    /// What the thing is, for someone who has not heard of it.
    pub what: String,

    /// Why this run wants it.
    pub why: String,

    /// Rough download size.
    pub approx_bytes: u64,
}

/// A step the first probe ruled out, held open because the run contains a step
/// that could change the answer.
///
/// It is neither settled nor pending: settling it would report a verdict the
/// run is about to invalidate, and putting it in `pending` would apply a step
/// whose own probe currently says no.
pub struct Deferred {
    /// The step itself, still live so it can be probed again and then applied.
    pub step: Box<dyn CoreImprovement>,

    /// What the first probe said. Kept because a step that is still ruled out
    /// after the second look has to report something, and the first reason is
    /// what the user would otherwise have seen.
    pub reason: String,

    /// The steps whose completion releases it. Every one of them is pending,
    /// so this list is never empty.
    pub waiting_on: Vec<ImprovementId>,
}

impl std::fmt::Debug for Deferred {
    /// Hand-written for the same reason `RunPlan`'s is: `CoreImprovement` is a
    /// trait object with no `Debug` bound.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Deferred")
            .field("step", &self.step.id())
            .field("reason", &self.reason)
            .field("waiting_on", &self.waiting_on)
            .finish()
    }
}

/// Everything a run learned by reading the system, and nothing it did.
///
/// Holding the probed steps, the dependency report, and what each step means to
/// install in one value is what lets the caller put every question to the user
/// in one pass. A question asked after the first package is installed is not a
/// real question, because the alternative has already been taken away.
pub struct RunPlan {
    /// Steps the probe already settled: already applied, not applicable, or
    /// skipped. Nothing left to decide about these.
    pub settled: Vec<StepReport>,

    /// Steps that would run, in order.
    pub pending: Vec<Box<dyn CoreImprovement>>,

    /// Steps the probe ruled out that a pending step may yet unlock. Their
    /// packages ride the same install screen as everybody else's, because a
    /// step promoted mid-run must not fetch anything nobody agreed to.
    pub deferred: Vec<Deferred>,

    /// What those steps need, and whether this system has it.
    pub preflight: PreflightReport,

    /// Packages the pending steps would install themselves, read off their own
    /// plans. Empty for every step that only writes files and sets kernel
    /// parameters, which is most of them.
    pub step_installs: Vec<(ImprovementId, PlannedInstall)>,

    /// Packages the pending steps wanted and found already on the machine, so
    /// the confirmation screen can show that a step is fetching less than its
    /// title suggests.
    pub step_present: Vec<String>,

    /// When planning started, so the finished report can time the whole run
    /// rather than only the part after the user answered.
    pub started: Instant,
}

impl RunPlan {
    /// Whether this run would put any software on the machine.
    #[must_use]
    pub fn installs_anything(&self) -> bool {
        self.preflight.needs_install() || !self.step_installs.is_empty()
    }

    /// Every package this run would install, from both routes, in the order a
    /// user would meet them.
    #[must_use]
    pub fn installs(&self, packages: PackageManagerKind) -> Vec<PlannedInstall> {
        let mut all = self.preflight.planned_installs(packages);
        all.extend(
            self.step_installs
                .iter()
                .map(|(_, install)| install.clone()),
        );
        all
    }

    /// The steps that would not run if the user refuses to install anything.
    #[must_use]
    pub fn steps_needing_install(&self) -> Vec<ImprovementId> {
        let mut steps = self.preflight.steps_needing_install();
        for (step, _) in &self.step_installs {
            if !steps.contains(step) {
                steps.push(step.clone());
            }
        }
        steps
    }

    /// The package names the pre-flight install would fetch, resolved for this
    /// distro. Does not include what a step installs itself, which that step
    /// does through its own `apply`.
    #[must_use]
    pub fn packages_to_install(&self, packages: PackageManagerKind) -> Vec<String> {
        self.preflight.packages_to_install(packages)
    }

    /// Packages the run already found on the machine, from both routes.
    #[must_use]
    pub fn already_present(&self, packages: PackageManagerKind) -> Vec<String> {
        let mut present: Vec<String> = self
            .preflight
            .present()
            .iter()
            .filter_map(|resolved| resolved.dependency.package_name(packages))
            .map(str::to_owned)
            .collect();
        for package in &self.step_present {
            if !present.contains(package) {
                present.push(package.clone());
            }
        }
        present
    }

    /// Whether the run would do anything at all.
    ///
    /// A held-open step counts: the run has something left to decide about it,
    /// which is the opposite of empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.deferred.is_empty()
    }

    /// Every step this run may still apply, held open ones included.
    ///
    /// The list the pre-flight resolver and the install screen both read, so a
    /// step that is promoted mid-run has already had its packages agreed to.
    #[must_use]
    pub fn considered(&self) -> Vec<&dyn CoreImprovement> {
        let pending = self.pending.iter().map(AsRef::as_ref);
        let deferred = self.deferred.iter().map(|held| held.step.as_ref());
        pending.chain(deferred).collect()
    }
}

impl std::fmt::Debug for RunPlan {
    /// Hand-written because `CoreImprovement` is a trait object with no `Debug`
    /// bound, and adding one would force it on every step for the sake of a
    /// test assertion.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunPlan")
            .field("settled", &self.settled)
            .field("pending", &self.pending.len())
            .field("deferred", &self.deferred)
            .field("preflight", &self.preflight)
            .field("step_installs", &self.step_installs)
            .finish()
    }
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
