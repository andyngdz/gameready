//! Raise `vm.max_map_count` so memory-hungry Proton titles can start.

use std::path::{Path, PathBuf};

use crate::exec::Cmd;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, ParseFailure,
    PlannedAction, Privilege, Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change, RunId};
use crate::steps::constants::{
    managed_header, PROC_SYS_VM, SYSCTL_BIN, SYSCTL_DROPIN, VM_MAX_MAP_COUNT,
};

/// The value SteamOS ships, `INT_MAX - 5`.
///
/// Some Proton titles map an enormous number of regions and simply fail to
/// start below this: DayZ, Hogwarts Legacy, and CS2 are the usual reports. Arch
/// raised its default to 1048576 in April 2024 and Fedora and Ubuntu sit there
/// too, which closes most of the gap but not all of it.
///
/// The cost of raising it is close to nothing. The parameter caps how many
/// mappings a process may hold, not how much memory it may use; a process that
/// does not map that many is unaffected.
const TARGET: u64 = 2_147_483_642;

/// Raises `vm.max_map_count` and makes the change survive a reboot.
#[derive(Debug, Default, Clone, Copy)]
pub struct MaxMapCount;

impl MaxMapCount {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.sysctl.max-map-count")
    }

    /// Where the kernel exposes the live value.
    #[must_use]
    pub fn runtime_path() -> PathBuf {
        Path::new(PROC_SYS_VM).join("max_map_count")
    }

    /// Reads the value the kernel currently reports.
    fn read_current(&self, runner: &dyn crate::exec::CommandRunner) -> Result<u64, StepError> {
        let path = Self::runtime_path();
        let raw = runner.read_to_string(&path).map_err(StepError::Exec)?;

        raw.trim()
            .parse::<u64>()
            .map_err(|source| StepError::Parse {
                what: VM_MAX_MAP_COUNT,
                path,
                source: ParseFailure::Integer(source),
            })
    }

    /// The drop-in file's contents, carrying the marker `doctor` looks for.
    ///
    /// The run id has to be the run, not the step: `doctor` uses it to tie a
    /// leftover file back to the invocation that created it when the journal
    /// has been deleted.
    fn dropin_contents(run: RunId) -> String {
        format!(
            "{header}\n\
             # Remove this file or run `gameready rollback` to revert.\n\
             {VM_MAX_MAP_COUNT} = {TARGET}\n",
            header = managed_header(Self::id_const(), run),
        )
    }
}

impl Improvement for MaxMapCount {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Raise vm.max_map_count for Proton titles"
    }

    fn short_name(&self) -> &str {
        "vm.max_map_count"
    }

    fn blurb(&self) -> &str {
        "Memory maps for Proton titles"
    }

    fn gains(&self) -> Option<&str> {
        Some("Proton games that need many memory regions start, instead of refusing to.")
    }

    fn rationale(&self) -> &str {
        "Some Proton games map far more memory regions than the kernel default \
         allows and refuse to start. Raising the cap to the value SteamOS uses \
         costs nothing: it limits how many mappings a process may hold, not how \
         much memory it may use."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Memory, Tag::Wine]
    }
}

impl CoreImprovement for MaxMapCount {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let current = self.read_current(cx.runner)?;
        if current >= TARGET {
            return Ok(Probe::AlreadyApplied {
                evidence: format!("{VM_MAX_MAP_COUNT} is already {current}"),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let current = self.read_current(cx.runner)?;
        Ok(StepPlan::new(
            self.id(),
            format!("{VM_MAX_MAP_COUNT} {current} -> {TARGET}"),
        )
        .action(PlannedAction::CreateFile {
            path: SYSCTL_DROPIN.to_owned(),
            contents: format!("{VM_MAX_MAP_COUNT} = {TARGET}"),
        })
        .action(PlannedAction::SetSysctl {
            key: VM_MAX_MAP_COUNT.to_owned(),
            from: current.to_string(),
            to: TARGET.to_string(),
        }))
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let previous = self.read_current(cx.reader())?;
        let dropin = PathBuf::from(SYSCTL_DROPIN);
        let contents = Self::dropin_contents(cx.run());

        // Persistence first. If the run dies between the two mutations the
        // system is left correct on the next boot rather than correct now and
        // wrong later, which is the harder failure to notice.
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
                key: VM_MAX_MAP_COUNT.to_owned(),
                previous: previous.to_string(),
            },
            |runner| {
                let set = Cmd::root(SYSCTL_BIN)
                    .arg("-w")
                    .arg(format!("{VM_MAX_MAP_COUNT}={TARGET}"));
                runner.run(&set).map(|_| ()).map_err(StepError::Exec)
            },
        )
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let current = self.read_current(cx.runner)?;
        let persisted = cx.runner.path_exists(Path::new(SYSCTL_DROPIN));

        Ok(Verification::new()
            .check(Check::equals(
                format!("runtime {VM_MAX_MAP_COUNT}"),
                TARGET.to_string(),
                current.to_string(),
            ))
            .check(Check::equals(
                format!("{SYSCTL_DROPIN} exists"),
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
#[path = "sysctl_max_map_count_test.rs"]
mod sysctl_max_map_count_test;
