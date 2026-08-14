//! What a run worked out about itself before changing anything.

use std::time::Instant;

use crate::facts::PackageManagerKind;
use crate::improvement::{CoreImprovement, ImprovementId, Privilege};
use crate::run::domain::contested::{self, Contested};
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
            .field(contested::STEP_FIELD, &self.step.id())
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

    /// Steps the probe found in conflict, where the run can take the seat back
    /// if the user says so. Their packages are never a concern: something is
    /// already running, which is proof the software that runs it is here.
    pub contested: Vec<Contested>,

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
    /// How many steps the sweep will report on.
    ///
    /// Held-open steps are counted: one way or another every one of them ends
    /// with a line, either because it was released and ran or because whatever
    /// it waited on never came. Contested steps count too: the sweep ends each
    /// with a row whether the user agreed to the takeover or not.
    #[must_use]
    pub fn to_apply(&self) -> usize {
        self.pending.len() + self.deferred.len() + self.contested.len()
    }

    /// Whether anything this run may apply reaches outside the user's files.
    ///
    /// Held-open steps count: one of them being released is not a reason to
    /// stop and ask for a password half way through a run that has already
    /// started changing things. A contested step counts the same way: its
    /// takeover stops and restarts services, which is root's work.
    #[must_use]
    pub fn needs_root(&self) -> bool {
        self.considered()
            .iter()
            .any(|step| matches!(step.privilege(), Privilege::Root))
            || self
                .contested
                .iter()
                .any(|entry| matches!(entry.step.privilege(), Privilege::Root))
    }

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
    /// which is the opposite of empty. A contested step counts the same way:
    /// the takeover question is a decision this run still has to make.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty() && self.deferred.is_empty() && self.contested.is_empty()
    }

    /// Every step this run may still apply, held open ones included.
    ///
    /// The list the pre-flight resolver and the install screen both read, so a
    /// step that is promoted mid-run has already had its packages agreed to.
    /// Contested steps are left out: their packages are on the machine already
    /// (something is running), and nothing about them rides the install screen.
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
            .field("contested", &self.contested)
            .field("preflight", &self.preflight)
            .field("step_installs", &self.step_installs)
            .finish()
    }
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
