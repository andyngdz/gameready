//! Stop the kernel punishing a game that takes a split lock.

use std::path::{Path, PathBuf};

use crate::exec::Cmd;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change, RunId};
use crate::steps::constants::{
    KERNEL_SPLIT_LOCK_MITIGATE, PROC_SYS_KERNEL, SPLIT_LOCK_DROPIN, SYSCTL_BIN, UNDO_NO_REBOOT,
};
use crate::steps::domain::GAMEMODE;
use crate::steps::use_cases::sysctl_dropin::{read_value, single_key_dropin};

/// Warn about a split lock, but do not punish the thread that took one.
///
/// The kernel's other setting, 1, is what upstream calls misery mode: the
/// offending thread is held so nothing else on the machine pays for its
/// unaligned access. That is the right trade on a shared server and the wrong
/// one on a desktop, where the offending thread is the game.
const TARGET: u64 = 0;

/// Turns off the split-lock penalty and makes the change survive a reboot.
#[derive(Debug, Default, Clone, Copy)]
pub struct SplitLock;

impl SplitLock {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.sysctl.split-lock")
    }

    /// Where the kernel exposes the live value.
    #[must_use]
    pub fn runtime_path() -> PathBuf {
        Path::new(PROC_SYS_KERNEL).join("split_lock_mitigate")
    }

    /// Reads the value the kernel currently reports.
    fn read_current(&self, runner: &dyn crate::exec::CommandRunner) -> Result<u64, StepError> {
        read_value(runner, &Self::runtime_path(), KERNEL_SPLIT_LOCK_MITIGATE)
    }

    /// The drop-in file's contents, carrying the marker `doctor` looks for.
    fn dropin_contents(run: RunId) -> String {
        single_key_dropin(Self::id_const(), run, KERNEL_SPLIT_LOCK_MITIGATE, TARGET)
    }
}

impl Improvement for SplitLock {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Stop the split-lock penalty stalling games"
    }

    /// Short on purpose: the label column is sized from the widest name in the
    /// catalog, so a long one here wraps the evidence for every other step too.
    fn short_name(&self) -> &str {
        "split lock"
    }

    fn bar_name(&self) -> &str {
        "Split lock"
    }

    fn blurb(&self) -> &str {
        "Split-lock penalty for games"
    }

    fn gains(&self) -> Option<&str> {
        Some("Games that take split locks run at full speed instead of being throttled.")
    }

    fn undo_note(&self) -> Option<&str> {
        Some(UNDO_NO_REBOOT)
    }

    fn rationale(&self) -> &str {
        "A handful of games take split locks, an unaligned atomic access the \
         CPU handles slowly. Since kernel 5.19 the default is to hold the \
         offending thread so the rest of the machine does not pay for it, which \
         on a desktop throttles the game to a crawl. Turning the penalty off \
         keeps the kernel log warning and drops the stall. gamemode already \
         does this for anything launched through gamemoderun, so this step \
         stands down when gamemode is installed and exists for the games that \
         are not started that way."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Cpu]
    }
}

impl CoreImprovement for SplitLock {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        // Only x86 has a split-lock detector, and only some kernels build it.
        // Either way the file is absent and there is nothing to turn off.
        if !cx.runner.path_exists(&Self::runtime_path()) {
            return Ok(Probe::NotApplicable {
                reason: "this kernel has no split-lock detector".to_owned(),
            });
        }

        let current = self.read_current(cx.runner)?;
        if current == TARGET {
            return Ok(Probe::AlreadyApplied {
                evidence: format!("{KERNEL_SPLIT_LOCK_MITIGATE} is already {current}"),
            });
        }
        // gamemode ships disable_splitlock=1, so it already clears this while a
        // client runs and puts it back afterwards. Every launch option
        // gameready writes starts with gamemoderun, so on a machine that has
        // gamemode the games this tool configures are covered. Standing down
        // here matches what core.cpu.governor does for the same reason.
        if cx.runner.which(GAMEMODE.binary).is_some() {
            return Ok(Probe::AlreadyApplied {
                evidence: "gamemode is here and clears it while a game runs".to_owned(),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let current = self.read_current(cx.runner)?;
        Ok(StepPlan::new(
            self.id(),
            format!("{KERNEL_SPLIT_LOCK_MITIGATE} {current} -> {TARGET}"),
        )
        .action(PlannedAction::CreateFile {
            path: SPLIT_LOCK_DROPIN.to_owned(),
            contents: format!("{KERNEL_SPLIT_LOCK_MITIGATE} = {TARGET}"),
        })
        .action(PlannedAction::SetSysctl {
            key: KERNEL_SPLIT_LOCK_MITIGATE.to_owned(),
            from: current.to_string(),
            to: TARGET.to_string(),
        }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let previous = self.read_current(cx.reader())?;
        let dropin = PathBuf::from(SPLIT_LOCK_DROPIN);
        let contents = Self::dropin_contents(cx.run());

        // Persistence first, for the reason given on the max_map_count step: a
        // run that dies between the two leaves the machine correct on the next
        // boot rather than correct now and wrong later.
        let sha256_after = digest(&contents);
        cx.mutate(
            Change::FileWritten {
                path: dropin.clone(),
                existed: false,
                backup: None,
                sha256_after,
                mode: 0o644,
                privilege: Privilege::Root,
            },
            |runner| {
                runner
                    .write_file(&dropin, &contents, Privilege::Root)
                    .map_err(StepError::Exec)
            },
        )?;

        cx.mutate(
            Change::SysctlRuntime {
                key: KERNEL_SPLIT_LOCK_MITIGATE.to_owned(),
                previous: previous.to_string(),
            },
            |runner| {
                let set = Cmd::root(SYSCTL_BIN)
                    .arg("-w")
                    .arg(format!("{KERNEL_SPLIT_LOCK_MITIGATE}={TARGET}"));
                runner.run(&set).map(|_| ()).map_err(StepError::Exec)
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let current = self.read_current(cx.runner)?;
        let persisted = cx.runner.path_exists(Path::new(SPLIT_LOCK_DROPIN));

        Ok(Verification::new()
            .check(Check::equals(
                format!("runtime {KERNEL_SPLIT_LOCK_MITIGATE}"),
                TARGET.to_string(),
                current.to_string(),
            ))
            .check(Check::equals(
                format!("{SPLIT_LOCK_DROPIN} exists"),
                "yes",
                if persisted { "yes" } else { "no" },
            )))
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // Reverse order: the runtime value goes back before the drop-in is
        // removed, so an interrupted rollback never leaves a file claiming a
        // value the kernel no longer has.
        for change in undo.iter().rev() {
            match change {
                Change::SysctlRuntime { key, previous } => {
                    let restore = Cmd::root(SYSCTL_BIN)
                        .arg("-w")
                        .arg(format!("{key}={previous}"));
                    cx.reader().run(&restore).map_err(StepError::Exec)?;
                }
                Change::FileWritten { path, .. } => {
                    cx.reader()
                        .remove_file(path, Privilege::Root)
                        .map_err(StepError::Exec)?;
                }
                // Listed rather than wildcarded: a new Change variant this
                // step starts recording must fail to compile here rather than
                // be silently skipped by rollback.
                Change::FileRemoved { .. }
                | Change::SysfsWrite { .. }
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
#[path = "sysctl_split_lock_test.rs"]
mod sysctl_split_lock_test;
