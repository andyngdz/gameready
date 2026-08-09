//! Rendering `gameready doctor`: what the machine is, and what each tuning
//! would do, changing nothing.

use std::fmt;

use console::style;
use gameready_core::doctor::{MachineReport, StepFinding, Warning};
use gameready_core::facts::SystemFacts;

use crate::cli::ui::layout::{Mark, ResultTable, Section};
use crate::cli::ui::{name_column, short_names};

/// The word a swap-less or disk-less machine shows.
const NONE: &str = "none";

/// The label in front of what the user could do about a warning.
const FIX: &str = "Fix";

/// The closing line: what this screen did not do, and what does.
const NOTHING_CHANGED: &str = "Nothing above has been changed. Run gameready init to apply.";

/// The whole doctor screen, built from borrowed facts and probe results.
pub struct DoctorReport<'a> {
    facts: &'a SystemFacts,
    machine: &'a MachineReport,
    findings: &'a [StepFinding],
    warnings: &'a [Warning],
}

impl<'a> DoctorReport<'a> {
    #[must_use]
    pub const fn new(
        facts: &'a SystemFacts,
        machine: &'a MachineReport,
        findings: &'a [StepFinding],
        warnings: &'a [Warning],
    ) -> Self {
        Self {
            facts,
            machine,
            findings,
            warnings,
        }
    }

    /// The "Your machine" block: what a user reads to recognise their own box.
    fn machine_block<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        s.heading(&style("Your machine").bold().to_string())?;
        s.labelled("distro", &self.facts.distro.name)?;
        s.labelled(
            "family",
            &format!(
                "{} · {}",
                self.facts.distro.family,
                self.facts.distro.package_manager()
            ),
        )?;
        let sched_ext = if self.machine.sched_ext_ready {
            "sched_ext ready"
        } else {
            "no sched_ext"
        };
        s.labelled(
            "kernel",
            &format!("{} · {sched_ext}", self.facts.kernel_release),
        )?;
        s.labelled("swap", &self.swap_line())?;
        s.labelled("disks", &self.disks_line())
    }

    /// The per-tuning rows: a mark for what probing found, the short name, and
    /// the finding inline.
    fn tunings_block<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        s.heading(&style("What each tuning would do here").bold().to_string())?;
        let mut table = ResultTable::new(name_column(&short_names()));
        for finding in self.findings {
            let mark = Mark::for_status(finding.status());
            table.row(mark, &finding.short_name, &finding.note());
        }
        s.heading(&table.to_string())
    }

    /// The warnings, if any: something the user should act on but gameready will
    /// not decide for them.
    fn worth_knowing<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        if self.warnings.is_empty() {
            return Ok(());
        }
        s.blank()?;
        s.heading(&style("Worth knowing").yellow().bold().to_string())?;
        for warning in self.warnings {
            s.marked(Mark::Warning, &warning.finding)?;
            s.sub(&style(&warning.explanation).dim().to_string())?;
            s.sub(&format!(
                "{} {}",
                style(FIX).dim(),
                style(&warning.suggestion).bold()
            ))?;
        }
        Ok(())
    }

    /// The active swap as backing plus total size, or `none`.
    fn swap_line(&self) -> String {
        self.machine.swap.map_or_else(
            || NONE.to_owned(),
            |swap| {
                format!(
                    "{}, {}",
                    swap.backing.describe(),
                    Self::human_size(swap.total_kib)
                )
            },
        )
    }

    /// Every tunable disk with its current scheduler, or `none`.
    fn disks_line(&self) -> String {
        self.machine
            .disks
            .iter()
            .map(|disk| format!("{} · {}", disk.name, disk.scheduler))
            .reduce(|joined, next| format!("{joined}, {next}"))
            .unwrap_or_else(|| NONE.to_owned())
    }

    /// A size in the rough units the screen shows: whole gigabytes, or
    /// megabytes below a gigabyte. A line to recognise a machine by, not a
    /// figure anything depends on.
    fn human_size(kib: u64) -> String {
        const KIB_PER_GIB: u64 = 1024 * 1024;
        if kib >= KIB_PER_GIB {
            format!("{} GB", (kib as f64 / KIB_PER_GIB as f64).round() as u64)
        } else {
            format!("{} MB", (kib as f64 / 1024.0).round() as u64)
        }
    }
}

impl fmt::Display for DoctorReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.blank()?;
        self.machine_block(&mut s)?;
        s.blank()?;
        self.tunings_block(&mut s)?;
        self.worth_knowing(&mut s)?;
        s.blank()?;
        s.indented(&style(NOTHING_CHANGED).dim().to_string())
    }
}

#[cfg(test)]
#[path = "doctor_test.rs"]
mod doctor_test;
