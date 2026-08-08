//! Raise `vm.swappiness` when zram is the primary swap, and only then.

use std::path::{Path, PathBuf};

use crate::exec::{Cmd, CommandRunner};
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, ParseFailure,
    PlannedAction, Privilege, Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change, RunId};
use crate::steps::constants::{
    managed_header, PROC_SWAPS, PROC_SYS_VM, SWAPPINESS_DROPIN, SYSCTL_BIN, VM_SWAPPINESS,
};
use crate::steps::domain::{parse_proc_swaps, primary_is_zram, SwapArea};

/// The value zram guidance settles on. A swapped page costs a compress into RAM
/// rather than a disk seek, so the kernel should reach for swap early. Kernel
/// 5.8 lifted the ceiling to 200; 180 is what CachyOS and Fedora's zram setups
/// use.
///
/// Never lowered toward 1. That advice predates zram and works against it: it
/// tells the kernel to hoard pages in uncompressed RAM instead of letting zram
/// reclaim them.
const TARGET: u16 = 180;

/// Raises `vm.swappiness` for a zram-backed system and persists the change.
#[derive(Debug, Default, Clone, Copy)]
pub struct Swappiness;

impl Swappiness {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.memory.swappiness")
    }

    /// Where the kernel exposes the live value.
    #[must_use]
    pub fn runtime_path() -> PathBuf {
        Path::new(PROC_SYS_VM).join("swappiness")
    }

    /// The active swap areas, empty when `/proc/swaps` cannot be read: an
    /// unknown swap layout means `NotApplicable`, not a guess at swappiness.
    fn read_swaps(&self, runner: &dyn CommandRunner) -> Vec<SwapArea> {
        runner
            .read_to_string(Path::new(PROC_SWAPS))
            .ok()
            .map(|raw| parse_proc_swaps(&raw))
            .unwrap_or_default()
    }

    /// Reads the value the kernel currently reports.
    fn read_current(&self, runner: &dyn CommandRunner) -> Result<u16, StepError> {
        let path = Self::runtime_path();
        let raw = runner.read_to_string(&path).map_err(StepError::Exec)?;

        raw.trim()
            .parse::<u16>()
            .map_err(|source| StepError::Parse {
                what: VM_SWAPPINESS,
                path,
                source: ParseFailure::Integer(source),
            })
    }

    /// The drop-in file's contents, carrying the marker `doctor` looks for.
    fn dropin_contents(run: RunId) -> String {
        format!(
            "{header}\n\
             # Remove this file or run `gameready rollback` to revert.\n\
             {VM_SWAPPINESS} = {TARGET}\n",
            header = managed_header(Self::id_const(), run),
        )
    }
}

impl Improvement for Swappiness {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Raise vm.swappiness for zram swap"
    }

    fn short_name(&self) -> &str {
        "Swappiness"
    }

    fn blurb(&self) -> &str {
        "Swappiness for zram"
    }

    fn gains(&self) -> Option<&str> {
        Some("More usable memory under pressure, at almost no cost, on a zram system.")
    }

    fn rationale(&self) -> &str {
        "When swap lives in zram, a swapped-out page is compressed into RAM \
         rather than written to disk, so swapping early frees more usable memory \
         at almost no cost. This raises swappiness to the value zram setups use, \
         and only when zram is the swap the kernel fills first."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Memory]
    }
}

impl CoreImprovement for Swappiness {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        if !primary_is_zram(&self.read_swaps(cx.runner)) {
            return Ok(Probe::NotApplicable {
                reason: "swap is on disk, not zram; the default swappiness is right".to_owned(),
            });
        }
        let current = self.read_current(cx.runner)?;
        if current >= TARGET {
            return Ok(Probe::AlreadyApplied {
                evidence: format!("{VM_SWAPPINESS} is already {current}"),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let current = self.read_current(cx.runner)?;
        Ok(
            StepPlan::new(self.id(), format!("{VM_SWAPPINESS} {current} -> {TARGET}"))
                .action(PlannedAction::CreateFile {
                    path: SWAPPINESS_DROPIN.to_owned(),
                    contents: format!("{VM_SWAPPINESS} = {TARGET}"),
                })
                .action(PlannedAction::SetSysctl {
                    key: VM_SWAPPINESS.to_owned(),
                    from: current.to_string(),
                    to: TARGET.to_string(),
                }),
        )
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let previous = self.read_current(cx.reader())?;
        let dropin = PathBuf::from(SWAPPINESS_DROPIN);
        let contents = Self::dropin_contents(cx.run());

        // Persistence first. If the run dies between the two mutations the system
        // is left correct on the next boot rather than correct now and wrong
        // later, which is the harder failure to notice.
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
                key: VM_SWAPPINESS.to_owned(),
                previous: previous.to_string(),
            },
            |runner| {
                let set = Cmd::root(SYSCTL_BIN)
                    .arg("-w")
                    .arg(format!("{VM_SWAPPINESS}={TARGET}"));
                runner.run(&set).map(|_| ()).map_err(StepError::Exec)
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let current = self.read_current(cx.runner)?;
        let persisted = cx.runner.path_exists(Path::new(SWAPPINESS_DROPIN));

        Ok(Verification::new()
            .check(Check::equals(
                format!("runtime {VM_SWAPPINESS}"),
                TARGET.to_string(),
                current.to_string(),
            ))
            .check(Check::equals(
                format!("{SWAPPINESS_DROPIN} exists"),
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
                // Listed rather than wildcarded: a new Change variant this step
                // records must fail to compile here, not be skipped silently.
                Change::FileRemoved { .. }
                | Change::SysfsWrite { .. }
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
#[path = "memory_swappiness_test.rs"]
mod memory_swappiness_test;
