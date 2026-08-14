//! Shorten the worst case when a running game asks the kernel for memory.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use itertools::Itertools as _;

use crate::exec::Cmd;
use crate::improvement::{
    ApplyCx, Check, CoreCx, CoreImprovement, Improvement, ImprovementId, PlannedAction, Privilege,
    Probe, StepError, StepPlan, Tag, Verification,
};
use crate::journal::{digest, Change, RunId};
use crate::steps::constants::{managed_header, SYSCTL_BIN, UNDO_NO_REBOOT, VM_LATENCY_DROPIN};
use crate::steps::use_cases::sysctl_vm_latency_survey::{survey, KnobReading};

/// Retunes the memory manager for latency and makes it survive a reboot.
#[derive(Debug, Default, Clone, Copy)]
pub struct VmLatency;

impl VmLatency {
    /// The step's stable id.
    #[must_use]
    pub const fn id_const() -> ImprovementId {
        ImprovementId::from_static("core.sysctl.vm-latency")
    }

    /// The drop-in file's contents, carrying the marker `doctor` looks for.
    ///
    /// Only the parameters this kernel actually has are written. A drop-in
    /// naming a parameter the kernel does not know makes `sysctl --system`
    /// report an error on every boot.
    fn dropin_contents(readings: &[KnobReading], run: RunId) -> String {
        let mut contents = format!(
            "{header}\n# Remove this file or run `gameready rollback` to revert.\n",
            header = managed_header(Self::id_const(), run),
        );
        for reading in readings {
            let _ = writeln!(
                contents,
                "\n# {why}\n{key} = {target}",
                why = reading.knob.why,
                key = reading.knob.key,
                target = reading.knob.target,
            );
        }
        contents
    }
}

impl Improvement for VmLatency {
    fn id(&self) -> ImprovementId {
        Self::id_const()
    }

    fn name(&self) -> &str {
        "Retune memory reclaim for shorter stalls"
    }

    fn short_name(&self) -> &str {
        "vm latency"
    }

    fn bar_name(&self) -> &str {
        "Memory latency"
    }

    fn blurb(&self) -> &str {
        "Memory manager latency"
    }

    fn gains(&self) -> Option<&str> {
        Some("Fewer stutters when a game allocates memory mid-scene. No effect on average frame rate.")
    }

    fn undo_note(&self) -> Option<&str> {
        Some(UNDO_NO_REBOOT)
    }

    fn rationale(&self) -> &str {
        "The kernel's memory manager is tuned for throughput on a server: it \
         defragments in the background and lets writeback pile up before \
         flushing. Both are cheap on average and expensive in the worst case, \
         and a game only ever notices the worst case. These five parameters \
         trade a little background housekeeping for a shorter wait when a game \
         asks for a page. None of them hands out more memory or raises a limit."
    }

    fn privilege(&self) -> Privilege {
        Privilege::Root
    }

    fn tags(&self) -> &[Tag] {
        &[Tag::Memory]
    }
}

impl CoreImprovement for VmLatency {
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError> {
        let readings = survey(cx.runner)?;

        let Some(first) = readings.first() else {
            return Ok(Probe::NotApplicable {
                reason: "this kernel exposes none of these parameters".to_owned(),
            });
        };

        if readings.iter().all(KnobReading::already_set) {
            return Ok(Probe::AlreadyApplied {
                evidence: format!(
                    "{key} is {current} (all {count} at target)",
                    count = readings.len(),
                    key = first.knob.key,
                    current = first.current,
                ),
            });
        }
        Ok(Probe::Applicable)
    }

    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError> {
        let readings = survey(cx.runner)?;
        let pending = readings
            .iter()
            .filter(|reading| !reading.already_set())
            .count();

        let mut plan = StepPlan::new(
            self.id(),
            format!("{pending} of {} parameters retuned", readings.len()),
        )
        .action(PlannedAction::CreateFile {
            path: VM_LATENCY_DROPIN.to_owned(),
            contents: readings
                .iter()
                .map(|reading| format!("{} = {}", reading.knob.key, reading.knob.target))
                .join("\n"),
        });

        for reading in &readings {
            plan = plan.action(PlannedAction::SetSysctl {
                key: reading.knob.key.to_owned(),
                from: reading.current.clone(),
                to: reading.knob.target.to_owned(),
            });
        }
        Ok(plan)
    }

    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        let readings = survey(cx.reader())?;
        let dropin = PathBuf::from(VM_LATENCY_DROPIN);
        let contents = Self::dropin_contents(&readings, cx.run());

        // Persistence first, so a run that dies partway leaves the machine
        // correct on the next boot rather than correct now and wrong later.
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

        for reading in &readings {
            let key = reading.knob.key;
            let target = reading.knob.target;
            cx.mutate(
                Change::SysctlRuntime {
                    key: key.to_owned(),
                    previous: reading.current.clone(),
                },
                |runner| {
                    let set = Cmd::root(SYSCTL_BIN)
                        .arg("-w")
                        .arg(format!("{key}={target}"));
                    runner.run(&set).map(|_| ()).map_err(StepError::Exec)
                },
            )?;
        }
        Ok(())
    }

    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError> {
        let readings = survey(cx.runner)?;
        let persisted = cx.runner.path_exists(Path::new(VM_LATENCY_DROPIN));

        let mut verification = Verification::new().check(Check::equals(
            format!("{VM_LATENCY_DROPIN} exists"),
            "yes",
            if persisted { "yes" } else { "no" },
        ));
        for reading in &readings {
            verification = verification.check(Check::equals(
                format!("runtime {}", reading.knob.key),
                reading.knob.target,
                reading.current.clone(),
            ));
        }
        Ok(verification)
    }

    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        // Reverse order: every runtime value goes back before the drop-in is
        // removed, so an interrupted rollback never leaves a file claiming
        // values the kernel no longer has.
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
#[path = "sysctl_vm_latency_test.rs"]
mod sysctl_vm_latency_test;
