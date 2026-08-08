//! Pin the CPU governor to performance, but only when nothing else will.

use std::path::{Path, PathBuf};

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change, RunId};
use crate::steps::constants::managed_header;
use crate::steps::domain::GAMEMODE;
use crate::steps::use_cases::cpu_governor_policies::{
    conflicting_daemon, read_policies, summary, GovernorPolicy, CPU_GOVERNOR_RULE,
    PERFORMANCE_GOVERNOR,
};
use crate::steps::use_cases::GamingTools;

/// The udev rule body that re-pins the governor on every boot.
const RULE_BODY: &str = r#"SUBSYSTEM=="cpu", ATTR{cpufreq/scaling_governor}="performance""#;

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

    /// The rule file's contents, carrying the marker `doctor` looks for.
    fn rule_contents(run: RunId) -> String {
        format!(
            "{header}\n{RULE_BODY}\n",
            header = managed_header(Self::id_const(), run),
        )
    }
}

impl Improvement for CpuGovernor {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Pin the CPU governor to performance"
    }

    fn short_name(&self) -> &str {
        "CPU governor"
    }

    fn blurb(&self) -> &str {
        "The CPU governor, when nothing else raises it"
    }

    fn rationale(&self) -> &str {
        "A frame is late when the CPU is still ramping its clocks as it arrives. \
         The performance governor holds them up so it does not. gamemode does \
         this per game and lowers them again afterwards, which is the better \
         deal on a laptop, so this step stands aside wherever gamemode is \
         present. It pins the governor itself only when nothing else will, and \
         only for this boot unless you ask it to persist."
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
                reason: "this machine reports no CPU governor to set".to_owned(),
            });
        }
        if policies.iter().all(GovernorPolicy::is_performance) {
            return Ok(Probe::AlreadyApplied {
                evidence: "every CPU policy is already on performance".to_owned(),
            });
        }
        if !policies.iter().all(GovernorPolicy::offers_performance) {
            return Ok(Probe::NotApplicable {
                reason: "this hardware offers no performance governor".to_owned(),
            });
        }
        // Before the gamemode check: a live daemon overwrites gamemode too.
        if let Some(daemon) = conflicting_daemon(cx.runner) {
            return Ok(Probe::Conflict {
                with: daemon.to_owned(),
                detail: format!(
                    "{daemon} sets the governor on its own schedule, so a pin here would be \
                     overwritten seconds later"
                ),
                yours: Some(format!("systemctl disable --now {daemon}")),
            });
        }
        if cx.runner.which(GAMEMODE.binary).is_some() {
            return Ok(Probe::AlreadyApplied {
                evidence: "gamemode is here and raises the governor while a game runs".to_owned(),
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
                contents: RULE_BODY.to_owned(),
            });
        }
        Ok(plan)
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        cx.progress("Setting the CPU governor");
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
            let contents = Self::rule_contents(cx.run());
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
                | Change::AptRepository { .. }
                | Change::ScxScheduler { .. }
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
