//! Set each disk's I/O scheduler to the one that suits its hardware.

use std::path::{Path, PathBuf};

use itertools::Itertools as _;

use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{Change, RunId, digest};
use crate::steps::constants::{IO_SCHEDULER_RULE, managed_header};
use crate::steps::use_cases::io_scheduler_devices::{DiskScheduler, scan_disks, summary};

/// The udev rules, one per disk class, that re-apply the choice on every boot.
///
/// NVMe by name; SATA and SAS disks by their rotational flag. Kept in step with
/// [`crate::steps::domain::BlockDevice::target_scheduler`]: the live write and
/// the persisted rule must choose the same scheduler or a reboot would silently
/// change it.
const RULE_LINES: [&str; 3] = [
    r#"ACTION=="add|change", KERNEL=="nvme[0-9]*", ATTR{queue/scheduler}="none""#,
    r#"ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="0", ATTR{queue/scheduler}="mq-deadline""#,
    r#"ACTION=="add|change", KERNEL=="sd[a-z]", ATTR{queue/rotational}=="1", ATTR{queue/scheduler}="bfq""#,
];

/// Sets each disk's I/O scheduler and makes the choice survive a reboot.
#[derive(Debug, Default, Clone, Copy)]
pub struct IoScheduler;

impl IoScheduler {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.io.scheduler")
    }

    /// The rule file's body, without the marker, for the plan screen.
    fn rule_body() -> String {
        RULE_LINES.join("\n")
    }

    /// The rule file's contents, carrying the marker `doctor` looks for.
    fn rule_contents(run: RunId) -> String {
        format!(
            "{header}\n{body}\n",
            header = managed_header(Self::id_const(), run),
            body = Self::rule_body(),
        )
    }
}

impl Improvement for IoScheduler {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Set each disk's I/O scheduler"
    }

    fn rationale(&self) -> &str {
        "The kernel's default I/O scheduler is a compromise across hardware. An \
         NVMe drive does best with none, since it queues in hardware; a SATA SSD \
         with mq-deadline; a spinning disk with bfq, whose fairness keeps the \
         system responsive while a game loads."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Io]
    }
}

impl CoreImprovement for IoScheduler {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let disks = scan_disks(cx.runner)?;
        if disks.is_empty() {
            return Ok(Probe::NotApplicable {
                reason: "no tunable block devices found".to_owned(),
            });
        }
        if disks.iter().any(DiskScheduler::needs_change) {
            return Ok(Probe::Applicable);
        }
        Ok(Probe::AlreadyApplied {
            evidence: disks
                .iter()
                .map(|disk| format!("{} on {}", disk.name, disk.state.active))
                .join(", "),
        })
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let disks = scan_disks(cx.runner)?;
        let mut plan =
            StepPlan::new(self.id(), summary(&disks)).action(PlannedAction::CreateFile {
                path: IO_SCHEDULER_RULE.to_owned(),
                contents: Self::rule_body(),
            });
        for disk in disks.iter().filter(|disk| disk.needs_change()) {
            plan = plan.action(PlannedAction::WriteSysfs {
                path: disk.scheduler_path.display().to_string(),
                from: disk.state.active.clone(),
                to: disk.target.to_owned(),
            });
        }
        Ok(plan)
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        cx.progress("Writing udev rule");
        let rule = PathBuf::from(IO_SCHEDULER_RULE);
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

        cx.progress("Setting schedulers");
        for disk in scan_disks(cx.reader())?
            .iter()
            .filter(|disk| disk.needs_change())
        {
            let path = disk.scheduler_path.clone();
            let target = disk.target;
            cx.mutate(
                Change::SysfsWrite {
                    path: path.clone(),
                    previous: disk.state.active.clone(),
                },
                |runner| {
                    runner
                        .write_sysfs(&path, target, Privilege::Root)
                        .map_err(StepError::Exec)
                },
            )?;
        }
        Ok(())
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let disks = scan_disks(cx.runner)?;
        let mut verification = Verification::new();
        for disk in disks.iter().filter(|disk| disk.state.offers(disk.target)) {
            verification = verification.check(Check::equals(
                format!("{} scheduler", disk.name),
                disk.target.to_owned(),
                disk.state.active.clone(),
            ));
        }
        Ok(verification.check(Check::equals(
            format!("{IO_SCHEDULER_RULE} exists"),
            "yes",
            if cx.runner.path_exists(Path::new(IO_SCHEDULER_RULE)) {
                "yes"
            } else {
                "no"
            },
        )))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // Reverse order: the live writes go back before the rule is removed, so
        // an interrupted rollback never leaves a rule claiming a scheduler the
        // disk no longer has.
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
#[path = "io_scheduler_test.rs"]
mod io_scheduler_test;
