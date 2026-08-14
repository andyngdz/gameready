//! Pin the CPU governor to performance, but only when nothing else will.

use std::path::{Path, PathBuf};

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change};
use crate::steps::domain::GAMEMODE;
use crate::steps::use_cases::cpu_governor_policies::{
    cpu_governor_rule, governor_conflict, read_policies, summary, GovernorPolicy,
    CPU_GOVERNOR_RULE, CPU_GOVERNOR_RULE_BODY, PERFORMANCE_GOVERNOR,
};
use crate::steps::use_cases::GamingTools;

/// The label every row shows for this step. One constant because the
/// terminal and the panel menu want the same words here.
const SHORT_NAME: &str = "CPU speed";

/// The step that pins the governor GamingTools may unlock into a skip.
///
/// A `static` rather than a `const`: a const is inlined at every use site, so
/// `requires` would hand back a reference to a temporary.
static UNLOCKED_BY: [ImprovementId; 1] = [GamingTools::id_const()];

/// Holds the CPU clocks up while a game runs, but only where nothing else does:
/// gamemode does it per game and better, so this steps in only where gamemode
/// is absent and no daemon already owns the governor.
#[derive(Debug, Default, Clone, Copy)]
pub struct CpuGovernor;

impl CpuGovernor {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.cpu.governor")
    }
}

impl Improvement for CpuGovernor {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Keep the CPU at full speed while you play"
    }

    fn short_name(&self) -> &str {
        SHORT_NAME
    }

    fn bar_name(&self) -> &str {
        "CPU speed"
    }

    fn blurb(&self) -> &str {
        "Full CPU speed, when nothing else asks for it"
    }

    fn rationale(&self) -> &str {
        "Linux holds the CPU at a low clock until it sees load, and the ramp \
         back up costs milliseconds a frame does not have. The kernel calls that \
         policy the governor, and its performance setting keeps the clocks up so \
         nothing waits on the ramp. gamemode does the same per game and lowers \
         them again afterwards, which is the better deal on a laptop, so this \
         step stands aside wherever gamemode is present. It holds the speed up \
         itself only when nothing else will, and only for this boot unless you \
         ask it to persist."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu]
    }

    /// gamemode arriving mid-run flips this to "gamemode has it"; re-probe then.
    fn requires(&self) -> &[ImprovementId] {
        &UNLOCKED_BY
    }
}

impl CoreImprovement for CpuGovernor {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let policies = read_policies(cx.runner);
        if policies.is_empty() {
            return Ok(Probe::NotApplicable {
                reason: "this machine reports no CPU speed setting to change".to_owned(),
            });
        }
        if policies.iter().all(GovernorPolicy::is_performance) {
            return Ok(Probe::AlreadyApplied {
                evidence: "every CPU is already at full speed".to_owned(),
            });
        }
        if !policies.iter().all(GovernorPolicy::offers_performance) {
            return Ok(Probe::NotApplicable {
                reason: "this hardware has no full-speed setting to hold".to_owned(),
            });
        }
        // A daemon that defeats gamemode (tuned) is still a conflict with
        // gamemode present; one gamemode drives (power-profiles-daemon) is not,
        // so `governor_conflict` skips it here and the run reads as handled.
        let gamemode_present = cx.runner.which(GAMEMODE.binary).is_some();
        if let Some(conflict) = governor_conflict(cx.runner, gamemode_present) {
            return Ok(conflict);
        }
        if gamemode_present {
            return Ok(Probe::AlreadyApplied {
                evidence: "gamemode is here and raises the CPU speed while a game runs".to_owned(),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let policies = read_policies(cx.runner);
        let mut plan = StepPlan::new(self.id(), summary(&policies));
        for policy in policies.iter().filter(|policy| policy.needs_change()) {
            plan = plan.action(PlannedAction::WriteSysfs {
                path: policy.governor_path.display().to_string(),
                from: policy.current.clone(),
                to: PERFORMANCE_GOVERNOR.to_owned(),
            });
        }
        if cx.governor_pinned {
            plan = plan.action(PlannedAction::CreateFile {
                path: CPU_GOVERNOR_RULE.to_owned(),
                contents: CPU_GOVERNOR_RULE_BODY.to_owned(),
            });
        }
        Ok(plan)
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        cx.progress("Setting the CPU speed");
        // Read at apply time, not from the probe: the value the undo has to put
        // back is the one that is there now.
        let changing: Vec<GovernorPolicy> = read_policies(cx.reader())
            .into_iter()
            .filter(GovernorPolicy::needs_change)
            .collect();
        for policy in changing {
            let path = policy.governor_path.clone();
            cx.mutate(
                Change::SysfsWrite {
                    path: path.clone(),
                    previous: policy.current.clone(),
                },
                |runner| {
                    runner
                        .write_sysfs(&path, PERFORMANCE_GOVERNOR, Privilege::Root)
                        .map_err(StepError::Exec)
                },
            )?;
        }

        if cx.cx.governor_pinned {
            cx.progress("Writing the boot rule");
            let rule = PathBuf::from(CPU_GOVERNOR_RULE);
            let contents = cpu_governor_rule(Self::id_const(), cx.run());
            let sha256_after = digest(&contents);
            cx.mutate(
                Change::FileWritten {
                    path: rule.clone(),
                    existed: false,
                    backup: None,
                    sha256_after,
                    mode: 0o644,
                    privilege: Privilege::Root,
                },
                |runner| {
                    runner
                        .write_file(&rule, &contents, Privilege::Root)
                        .map_err(StepError::Exec)
                },
            )?;
        }
        Ok(())
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let mut verification = Verification::new();
        for policy in read_policies(cx.runner)
            .iter()
            .filter(|policy| policy.offers_performance())
        {
            verification = verification.check(Check::equals(
                format!("{} governor", policy.name),
                PERFORMANCE_GOVERNOR.to_owned(),
                policy.current.clone(),
            ));
        }
        if cx.governor_pinned {
            verification = verification.check(Check::equals(
                format!("{CPU_GOVERNOR_RULE} exists"),
                "yes",
                if cx.runner.path_exists(Path::new(CPU_GOVERNOR_RULE)) {
                    "yes"
                } else {
                    "no"
                },
            ));
        }
        Ok(verification)
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // Reverse order: the live writes go back before the boot rule is
        // removed, so an interrupted rollback never leaves a rule claiming a
        // governor the machine is no longer on.
        for change in undo.iter().rev() {
            match change {
                Change::SysfsWrite { path, previous } => {
                    cx.reader()
                        .write_sysfs(path, previous, Privilege::Root)
                        .map_err(StepError::Exec)?;
                }
                Change::FileWritten { path, .. } => {
                    cx.reader()
                        .remove_file(path, Privilege::Root)
                        .map_err(StepError::Exec)?;
                }
                Change::FileRemoved { .. }
                | Change::SysctlRuntime { .. }
                | Change::PackagesInstalled { .. }
                | Change::SystemdUnit { .. }
                | Change::DirCreated { .. }
                | Change::DirTreeInstalled { .. } => {}
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "cpu_governor_test.rs"]
mod cpu_governor_test;
