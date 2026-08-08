//! Rendering `gameready doctor`: what the machine is, and what each tuning
//! would do, changing nothing.

use std::fmt;

use console::style;
use gameready_core::doctor::{MachineReport, Warning};
use gameready_core::facts::SystemFacts;
use gameready_core::improvement::{CoreCx, CoreImprovement, Probe};

use crate::cli::ui::layout::{Mark, Section};

/// The word a swap-less or disk-less machine shows.
const NONE: &str = "none";

/// The label in front of what the user could do about a warning.
const FIX: &str = "Fix";

/// The closing line: what this screen did not do, and what does.
const NOTHING_CHANGED: &str = "Nothing above has been changed. Run gameready init to apply.";

/// One tuning's probe result, ready to render: its short label and either what
/// probing found or why probing failed.
pub struct StepFinding {
    pub short_name: String,
    pub found: Result<Probe, String>,

    /// What the step would do, for the steps that would do something. A row
    /// reading only "would apply" tells the user this screen has an answer and
    /// then keeps it, which is the opposite of what they opened it for.
    pub would_do: Option<String>,
}

impl StepFinding {
    /// Probes one step, and asks a step that would run what it would do.
    ///
    /// The plan is worked out only for a step that would actually apply. It is
    /// the more expensive of the two calls, and for a step that is already set
    /// or ruled out it answers a question nobody asked.
    #[must_use]
    pub fn of(step: &dyn CoreImprovement, cx: &CoreCx<'_>) -> Self {
        let found = step.probe(cx).map_err(|error| error.describe());
        let would_do = matches!(found, Ok(Probe::Applicable))
            .then(|| step.plan(cx).ok().map(|plan| plan.summary))
            .flatten();
        Self {
            short_name: step.short_name().to_owned(),
            found,
            would_do,
        }
    }

    /// What this row says after the step's name.
    fn note(&self) -> String {
        let found = match &self.found {
            Ok(probe) => probe.describe(),
            Err(error) => format!("probe failed: {error}"),
        };
        match &self.would_do {
            Some(plan) => format!("{found}, {plan}"),
            None => found,
        }
    }
}

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
        for finding in self.findings {
            let mark = finding
                .found
                .as_ref()
                .map_or(Mark::Warning, Mark::for_probe);
            s.noted(mark, &finding.short_name, &finding.note())?;
        }
        Ok(())
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
