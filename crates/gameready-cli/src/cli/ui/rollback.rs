//! Rendering what a rollback did.

use std::fmt;
use std::path::Path;

use chrono::{DateTime, Local};
use console::style;
use gameready_core::journal::Undo;
use gameready_core::rollback::{RollbackReport, UndoOutcome, UndoReport};

use crate::cli::ui::layout::{Mark, ResultTable, Section};
use crate::cli::ui::widest;

/// A rollback report paired with where its journal lives, ready to print.
///
/// See [`crate::cli::ui::Summary`] for why this is a `Display` view rather
/// than a function returning `String`.
pub struct RollbackSummary<'a> {
    report: &'a RollbackReport,
    journal: &'a Path,
}

impl<'a> RollbackSummary<'a> {
    /// Pairs a report with the journal path shown at the bottom.
    #[must_use]
    pub const fn new(report: &'a RollbackReport, journal: &'a Path) -> Self {
        Self { report, journal }
    }

    /// When the run being undone started, in the reader's own time zone. The
    /// point of the line is to name a run by when it happened, so a raw id would
    /// defeat it.
    fn when(&self) -> String {
        let started: DateTime<Local> = self.report.run.started_at().into();
        started.format("%-d %b, %H:%M").to_string()
    }

    /// How the rollback ended, counted.
    ///
    /// A clean rollback does not carry "0 failed" along with it. The zero is
    /// the good news, and a user who has to read a number to find it out has
    /// been made to check rather than told.
    fn counts(&self) -> String {
        let reverted = style(format!("{} reverted", self.report.reverted())).bold();
        let failed = self.report.failed();
        if failed == 0 {
            return reverted.green().to_string();
        }
        format!("{reverted}, {}", style(format!("{failed} failed")).red())
    }

    /// One row per undo, except the package report, which is deliberately not a
    /// row: it did nothing to the system, so it belongs in the note below rather
    /// than in the list of things that were put back.
    fn rows<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        let subjects: Vec<String> = self.undone().map(|report| report.undo.subject()).collect();
        let mut table = ResultTable::new(widest(subjects.iter().map(String::as_str)));
        for (report, subject) in self.undone().zip(&subjects) {
            table.row(Self::mark(&report.outcome), subject, &Self::note(report));
        }
        s.heading(&table.to_string())
    }

    /// Every undo that stands for something that was put back.
    ///
    /// The package report is not one: it did nothing to the system, so it
    /// belongs in the note below rather than in the list of things undone.
    fn undone(&self) -> impl Iterator<Item = &UndoReport> {
        self.report
            .undos
            .iter()
            .filter(|report| !matches!(report.undo, Undo::ReportPackages { .. }))
    }

    /// The packages the run installed and rollback left in place, gathered from
    /// every package report into one note rather than one line each.
    fn packages<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        let installed: Vec<&str> = self
            .report
            .undos
            .iter()
            .filter_map(|report| {
                if let Undo::ReportPackages { installed, .. } = &report.undo {
                    Some(installed.as_slice())
                } else {
                    None
                }
            })
            .flatten()
            .map(String::as_str)
            .collect();
        if installed.is_empty() {
            return Ok(());
        }

        s.marked(
            Mark::Warning,
            &format!("{} are still installed.", join_and(&installed)),
        )?;
        s.sub("Remove them yourself if you want them gone.")?;
        s.blank()
    }

    /// The gutter mark for how one undo ended.
    fn mark(outcome: &UndoOutcome) -> Mark {
        match outcome {
            UndoOutcome::Reverted { .. } => Mark::Applied,
            UndoOutcome::AlreadyGone => Mark::AlreadySet,
            UndoOutcome::Left { .. } => Mark::Skipped,
            UndoOutcome::Refused { .. } => Mark::Warning,
            UndoOutcome::Failed { .. } => Mark::Failed,
        }
    }

    /// The note after the subject: what was put back, or why it was not.
    fn note(report: &UndoReport) -> String {
        match &report.outcome {
            UndoOutcome::Reverted { .. } => Self::reverted_note(&report.undo),
            UndoOutcome::AlreadyGone => "was already gone".to_owned(),
            UndoOutcome::Left { reason } | UndoOutcome::Refused { reason } => reason.clone(),
            UndoOutcome::Failed { error } => error.clone(),
        }
    }

    /// What a reverted operation reads as, phrased for the operation rather than
    /// repeating the subject the row already shows.
    fn reverted_note(undo: &Undo) -> String {
        match undo {
            Undo::SetSysctl { value, .. } | Undo::WriteSysfs { value, .. } => {
                format!("back to {value}")
            }
            Undo::RestoreFile { .. } => "restored from the copy taken first".to_owned(),
            Undo::RestoreUnit { .. } => "disabled again".to_owned(),
            Undo::DeleteFile { .. }
            | Undo::RemoveDirIfEmpty { .. }
            | Undo::RemoveDirTree { .. } => "removed".to_owned(),
            Undo::ReportPackages { .. } => String::new(),
        }
    }
}

impl fmt::Display for RollbackSummary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.blank()?;
        s.title(&format!("Putting back the run from {}", self.when()))?;
        self.rows(&mut s)?;

        s.blank()?;
        s.heading(&self.counts())?;
        s.blank()?;

        self.packages(&mut s)?;
        s.indented(
            &style(format!("journal · {}", self.journal.display()))
                .dim()
                .to_string(),
        )
    }
}

/// Joins names as a reader would say them: "a", "a and b", "a, b and c".
fn join_and(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

#[cfg(test)]
#[path = "rollback_test.rs"]
mod rollback_test;
