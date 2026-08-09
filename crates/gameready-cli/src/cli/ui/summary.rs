//! Rendering what a run did.

use std::fmt;
use std::path::Path;

use console::style;
use gameready_core::improvement::{Outcome, OutcomeKind, SkipReason};
use gameready_core::run::RunReport;
use itertools::Itertools as _;

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::rows::{copyable, undo, StepRow};
use crate::cli::ui::{name_column, short_names};

/// What the rollback block offers, in the words of someone reconsidering.
const CHANGED_YOUR_MIND: &str = "Changed your mind? This puts everything back.";

/// Where to go next when the machine is set up.
const GO_PLAY: &str = "Play something. Run gameready doctor if it feels off.";

/// What a dry run leaves the user to do.
const DROP_THE_FLAG: &str = "Nothing was touched. Drop --dry-run to apply.";

/// The full report printed after a run completes.
///
/// The one screen worth scrolling back to, so everything it says is either what
/// happened or what to type next.
pub struct Summary<'a> {
    report: &'a RunReport,
    journal: &'a Path,
}

impl<'a> Summary<'a> {
    #[must_use]
    pub const fn new(report: &'a RunReport, journal: &'a Path) -> Self {
        Self { report, journal }
    }

    /// The verdict, in the first three words a user will read.
    fn verdict(&self) -> String {
        if self.report.failed() > 0 {
            return style("Some of it did not land.").red().bold().to_string();
        }
        if !self.report.mode.mutates() {
            return style("Dry run.").dim().bold().to_string();
        }
        if self.report.applied() > 0 {
            return style("Your machine is set up.").green().bold().to_string();
        }
        style("Your machine was already set up.")
            .green()
            .bold()
            .to_string()
    }

    /// How the steps ended, counted by kind and in a fixed order, so two runs
    /// of the same machine read the same way.
    fn counts(&self) -> String {
        const ORDER: [OutcomeKind; 5] = [
            OutcomeKind::Applied,
            OutcomeKind::AlreadySet,
            OutcomeKind::Skipped,
            OutcomeKind::NotApplicable,
            OutcomeKind::Failed,
        ];
        let waiting = self.would_apply();
        let counted = ORDER.iter().filter_map(|kind| {
            let mut count = self.tally(*kind);
            if *kind == OutcomeKind::Skipped {
                count = count.saturating_sub(waiting);
            }
            (count > 0).then(|| format!("{count} {}", kind.label()))
        });
        (waiting > 0)
            .then(|| format!("{waiting} would apply"))
            .into_iter()
            .chain(counted)
            .join(" · ")
    }

    /// How many steps ended one way.
    fn tally(&self, kind: OutcomeKind) -> usize {
        self.report
            .steps
            .iter()
            .filter(|step| step.outcome.kind() == kind)
            .count()
    }

    /// Steps a dry run stopped short of.
    ///
    /// They are recorded as skips, which is true of the machine and no use to a
    /// reader: what they want off this screen is how much work dropping the
    /// flag would do, not how much this run declined to do.
    fn would_apply(&self) -> usize {
        self.report
            .steps
            .iter()
            .filter(|step| {
                matches!(
                    &step.outcome,
                    Outcome::Skipped {
                        reason: SkipReason::DryRun
                    }
                )
            })
            .count()
    }

    /// Whether this run wrote anything to the journal.
    ///
    /// Only an applied or failed step appends a record. Every other run leaves
    /// the journal exactly as it found it, so there is nothing to undo and
    /// nothing worth pointing a user at.
    fn recorded(&self) -> bool {
        self.report.applied() + self.report.failed() > 0
    }

    /// One row per step, each turned into its own line by `StepRow`.
    fn steps<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        let names = short_names();
        let column = name_column(&names);
        for step in &self.report.steps {
            let name = names
                .get(&step.step)
                .cloned()
                .unwrap_or_else(|| step.name.clone());
            StepRow {
                mark: Mark::of(step.outcome.kind()),
                name: &name,
                outcome: &step.outcome,
                column,
                run: &self.report.run,
            }
            .write(s)?;
        }
        Ok(())
    }

    /// The undo command and the record it reads back.
    fn rollback<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        s.indented(&style(CHANGED_YOUR_MIND).dim().to_string())?;
        s.indented(&copyable(&undo(&self.report.run)))?;
        s.indented(
            &style(format!("journal · {}", self.journal.display()))
                .dim()
                .to_string(),
        )
    }

    /// What to do now that the run is over.
    fn next_step(&self) -> String {
        if self.report.mode.mutates() {
            style(GO_PLAY).dim().to_string()
        } else {
            style(DROP_THE_FLAG).dim().to_string()
        }
    }
}

impl fmt::Display for Summary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);

        // Every block this is concatenated onto ends in a separator, and a
        // heading pressed straight against one reads as part of the block
        // above it.
        s.blank()?;
        s.title(&format!(
            "{} {}",
            self.verdict(),
            style(self.counts()).dim()
        ))?;
        self.steps(&mut s)?;
        s.blank()?;

        // A run that recorded nothing has no undo command to offer and no new
        // record to point at, so the block goes entirely rather than putting a
        // heading over two lines that contradict it.
        if self.recorded() {
            self.rollback(&mut s)?;
            s.blank()?;
        }

        s.indented(&self.next_step())?;
        s.end()
    }
}

#[cfg(test)]
#[path = "summary_test.rs"]
mod summary_test;
