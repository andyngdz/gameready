//! Hand CPU scheduling to scx_lavd while gaming.

use crate::exec::Cmd;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction,
    PlannedPackage, Privilege, Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::Change;
use crate::steps::constants::{LAVD_SCHEDULER, SCHED_EXT_STATE};
use crate::steps::domain::{restore_scheduler, SchedExt};
use crate::steps::use_cases::scx_lavd_loader::{takeover_stop, Loader};
use crate::steps::use_cases::scx_lavd_packages::{probe_tooling, ScxPackages};
use crate::steps::use_cases::scx_ppa::ScxPpa;
use crate::steps::use_cases::scx_state::read_sched_ext;
use crate::systemd::{DISABLE, ENABLE, NOW, SYSTEMCTL};

/// The label every row shows for this step. One constant because the
/// terminal and the panel menu want the same words here.
const SHORT_NAME: &str = "CPU scheduler scx_lavd";

/// Loads the gaming-oriented sched_ext scheduler.
///
/// A frame is not produced by one thread. It is a chain of them waking each
/// other, and the kernel's own scheduler cannot see the chain, so a thread the
/// rest are blocked on waits its ordinary turn. scx_lavd measures which threads
/// get waited on and runs those first.
#[derive(Debug, Default, Clone, Copy)]
pub struct ScxLavd;

impl ScxLavd {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.sched.scx-lavd")
    }
}

impl Improvement for ScxLavd {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Switch the CPU scheduler to scx_lavd"
    }

    fn short_name(&self) -> &str {
        SHORT_NAME
    }

    fn bar_name(&self) -> &str {
        "Kernel scheduler"
    }

    fn blurb(&self) -> &str {
        "The scx_lavd CPU scheduler"
    }

    fn gains(&self) -> Option<&str> {
        Some(
            "Steadier frame times when something else wants the CPU: a browser, a \
             voice chat, a build. On an otherwise idle machine expect no \
             difference, and on games that lean on one or two cores it has measured \
             slower than the default.",
        )
    }

    fn undo_note(&self) -> Option<&str> {
        Some("hands the CPU straight back, no reboot")
    }

    fn rationale(&self) -> &str {
        "A frame is a chain of threads waking each other, and the kernel's own \
         scheduler does not know the chain exists, so a thread the others are \
         blocked on waits its ordinary turn. scx_lavd measures which threads \
         get waited on and runs those first."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu]
    }

    /// On Ubuntu the packages only resolve once the PPA step has run, so a
    /// probe taken before it says no about a machine that is minutes from
    /// saying yes. Naming it here is what buys the second look.
    fn requires(&self) -> &[ImprovementId] {
        &UNLOCKED_BY
    }
}

/// The step that makes scx resolvable on apt systems.
///
/// A `static` rather than a `const`: a const is inlined at every use site, so
/// `requires` would hand back a reference to a temporary.
static UNLOCKED_BY: [ImprovementId; 1] = [ScxPpa::id_const()];

impl CoreImprovement for ScxLavd {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let state = read_sched_ext(cx.runner);
        match &state {
            SchedExt::Unsupported => Ok(Probe::NotApplicable {
                reason: format!("this kernel has no sched_ext; {SCHED_EXT_STATE} is not there"),
            }),

            SchedExt::Running { .. } if state.is_running(LAVD_SCHEDULER) => {
                Ok(Probe::AlreadyApplied {
                    evidence: format!("{LAVD_SCHEDULER} is already the scheduler"),
                })
            }

            // Somebody else loaded a scheduler. Replacing it would take over a
            // choice this run did not make, so the step stands down and says
            // what is there, unless the run can take the seat cleanly. That is
            // when gameready loaded or manages the only mechanism on this
            // machine: scxctl present, or the scx unit itself running. `yours`
            // then names the stop, and a run may ask the user whether to do it.
            SchedExt::Running { .. } => Ok(Probe::Conflict {
                with: state.describe().to_owned(),
                detail: format!(
                    "{} is already scheduling this machine; stop it first if you want {LAVD_SCHEDULER}",
                    state.describe()
                ),
                yours: takeover_stop(cx),
            }),

            SchedExt::Idle => probe_tooling(cx),
        }
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let loader = Loader::detect(cx);
        let lasts = if loader.survives_reboot() {
            "and on every boot after it"
        } else {
            "until the next reboot"
        };

        let plan = StepPlan::new(
            self.id(),
            format!("run {LAVD_SCHEDULER} in gaming mode, {lasts}"),
        )
        .action(PlannedAction::RunCommand {
            display: loader.describe(),
        });

        let Some(packages) = cx.packages else {
            return Ok(plan);
        };
        let survey = ScxPackages::read(cx, packages)?;
        let missing: Vec<PlannedPackage> = survey.to_install();
        if missing.is_empty() {
            return Ok(plan);
        }

        Ok(plan.action(PlannedAction::InstallPackages {
            packages: missing,
            already_present: survey.already_here(),
        }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        ScxPackages::install_missing(cx)?;

        // Detected after the install, not before: on a machine with neither
        // mechanism present the packages are what decide which one it gets.
        let loader = Loader::detect(&cx.cx);
        cx.progress(&format!("Loading {LAVD_SCHEDULER}"));
        loader.load(cx)
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let state = read_sched_ext(cx.runner);
        Ok(Verification::new().check(Check::equals(
            "kernel scheduler",
            LAVD_SCHEDULER,
            state.describe(),
        )))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // The drop-in goes away before anything starts the unit again, so a
        // unit handed back to its own enablement runs the scheduler its own
        // config names rather than ours.
        for change in undo.iter().rev() {
            if let Change::FileWritten { path, .. } = change {
                cx.reader()
                    .remove_file(path, Privilege::Root)
                    .map_err(StepError::Exec)?;
            }
        }

        for change in undo.iter().rev() {
            match change {
                Change::ScxScheduler { previous } => {
                    let back = restore_scheduler(previous.as_deref());
                    cx.reader().run(&back).map_err(StepError::Exec)?;
                }
                Change::SystemdUnit {
                    unit,
                    was_enabled: true,
                    ..
                } => {
                    // The unit was the user's own enablement; give it back
                    // running, now that the drop-in that re-pointed it is gone.
                    let start = Cmd::root(SYSTEMCTL).arg(ENABLE).arg(NOW).arg(unit);
                    cx.reader().run(&start).map_err(StepError::Exec)?;
                }
                Change::SystemdUnit { unit, .. } => {
                    let stop = Cmd::root(SYSTEMCTL).arg(DISABLE).arg(NOW).arg(unit);
                    cx.reader().run(&stop).map_err(StepError::Exec)?;
                }
                // Removing a package is not the inverse of installing one, so
                // the packages stay and the summary says so.
                Change::PackagesInstalled { .. } => {}
                // Already removed in the pass above, before the unit that
                // reads it is started again.
                Change::FileWritten { .. } => {}
                // Listed rather than wildcarded, so a new change this step
                // starts recording fails to compile here instead of being
                // silently skipped by rollback.
                Change::AptRepository { .. }
                | Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::SysfsWrite { .. }
                | Change::DirCreated { .. }
                | Change::DirTreeInstalled { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "scx_lavd_test.rs"]
mod scx_lavd_test;
