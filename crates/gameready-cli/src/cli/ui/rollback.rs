//! Rendering what a rollback did.

use std::fmt;
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Local};
use console::style;
use gameready_core::exec::CommandRunner;
use gameready_core::journal::{PriorUnitState, Undo};
use gameready_core::rollback::{
    PlannedUndo, RollbackPlan, RollbackReport, UndoOutcome, UndoReport,
};
use gameready_core::steam::{capture_block, PriorBlock, PriorSection};

use crate::cli::ui::layout::{Mark, ResultTable, Section};
use crate::cli::ui::{theme, widest};

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
            // "Already gone" is right for an undo that removes something and
            // wrong for one that puts something back, where nothing left to do
            // means it is already back.
            UndoOutcome::AlreadyGone => match report.undo {
                Undo::RestoreSteamConfig { .. } => "was already put back".to_owned(),
                Undo::DeleteFile { .. }
                | Undo::SetSysctl { .. }
                | Undo::WriteSysfs { .. }
                | Undo::ReportPackages { .. }
                | Undo::RestoreUnit { .. }
                | Undo::RemoveDirIfEmpty { .. }
                | Undo::RemoveDirTree { .. } => "was already gone".to_owned(),
            },
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
            // Says "your settings" because the row is about a file Steam owns,
            // and the point the user needs is that only their own entries moved.
            Undo::RestoreSteamConfig { .. } => "your settings put back".to_owned(),
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

/// Asks whether a rollback may close Steam.
///
/// The Steam steps restore keys inside files Steam holds in memory and writes
/// out when it exits, so the restore only sticks while Steam is stopped, and
/// closing a running game client is the user's call: Steam may be mid-download
/// or running a game. Steam is reopened when the rollback finishes.
///
/// Anything short of an explicit yes answers no, including a terminal that
/// cannot ask. An unattended rollback must never close Steam on its own.
pub fn confirm_steam_close() -> Result<bool> {
    if !console::user_attended_stderr() {
        return Ok(false);
    }
    let choice = theme::Asked::new(
        "Undoing this run has to close Steam",
        "It rewrites files Steam holds in memory, so the undo only sticks while \
         Steam is stopped. Steam is reopened when the rollback finishes.",
        "up down move · enter choose · esc cancels",
    )
    .one_of(vec!["Yes, close Steam and undo", "No, cancel rollback"])
    .prompt_skippable()?;
    Ok(choice == Some("Yes, close Steam and undo"))
}

/// Renders what a rollback will undo, shown before it happens.
///
/// The rollback report only appears afterwards, when the machine has already
/// changed. Reading the plan first, and the current values it will overwrite
/// where they can be read, turns the confirm into a decision rather than a
/// review of something already done.
pub fn preview(plan: &RollbackPlan, runner: &dyn CommandRunner) -> String {
    let rows: Vec<PreviewRow> = plan
        .undos
        .iter()
        .flat_map(|planned| preview_rows(planned, runner))
        .collect();
    if rows.is_empty() {
        return String::new();
    }

    let subjects: Vec<String> = rows.iter().map(|row| row.subject.clone()).collect();
    let column = widest(subjects.iter().map(String::as_str));

    let mut out = String::new();
    {
        let mut s = Section::new(&mut out);
        let _ = s.title("What will be undone");
        let mut table = ResultTable::new(column);
        for row in &rows {
            table.row(Mark::Chosen, &row.subject, &row.evidence);
        }
        let _ = s.heading(&table.to_string());
        let _ = s.blank();
    }
    out
}

/// One line of the rollback preview.
struct PreviewRow {
    /// What the undo touches, named for how a user talks about it.
    subject: String,
    /// What will happen, with the current value when it can be read.
    evidence: String,
}

/// One undo becomes one or more preview rows.
///
/// A Steam undo carries several blocks, and each becomes its own row so a user
/// picks out "Proton pin" from "Launch options" rather than reading a path.
fn preview_rows(planned: &PlannedUndo, runner: &dyn CommandRunner) -> Vec<PreviewRow> {
    let subject = |evidence: String| PreviewRow {
        subject: planned.undo.subject(),
        evidence,
    };
    match &planned.undo {
        Undo::RestoreSteamConfig { path, sections } => sections
            .iter()
            .map(|section| steam_row(runner, path, section))
            .collect(),

        Undo::SetSysctl { key, value } => vec![subject(sysctl_arrow(runner, key, value))],

        Undo::WriteSysfs { path, value } => {
            vec![subject(arrow(
                current(runner, path).as_deref(),
                Some(value),
            ))]
        }

        Undo::RestoreUnit { unit, prior } => {
            let action = match prior {
                PriorUnitState::WasEnabled => format!("restart {unit} on its own config"),
                PriorUnitState::WasDisabled => format!("disable {unit}"),
            };
            vec![subject(action)]
        }

        Undo::ReportPackages { installed, .. } => {
            vec![subject(format!("keep installed: {}", installed.join(", ")))]
        }

        Undo::DeleteFile { path, .. } | Undo::RemoveDirTree { path, .. } => {
            vec![subject(format!("remove {}", path.display()))]
        }

        Undo::RemoveDirIfEmpty { path, .. } => {
            vec![subject(format!("remove {} if it is empty", path.display()))]
        }
    }
}

/// One block a Steam undo restores, named and with its current value.
fn steam_row(runner: &dyn CommandRunner, path: &Path, section: &PriorSection) -> PreviewRow {
    let subject = match steam_app(&section.section) {
        Some(id) => format!("{} · {id}", steam_label(section)),
        None => steam_label(section).to_owned(),
    };
    let target = match &section.prior {
        PriorBlock::Absent => None,
        PriorBlock::Present { entries } => entries.first().and_then(|entry| entry.value.clone()),
    };
    PreviewRow {
        subject,
        evidence: arrow(
            steam_current(runner, path, section).as_deref(),
            target.as_deref(),
        ),
    }
}

/// The current value a recorded Steam key holds on disk, when it can be read.
fn steam_current(
    runner: &dyn CommandRunner,
    path: &Path,
    section: &PriorSection,
) -> Option<String> {
    let key = match &section.prior {
        PriorBlock::Present { entries } => entries.first()?.key.as_str(),
        PriorBlock::Absent => return None,
    };
    let text = runner.read_to_string(path).ok()?;
    let visit: Vec<&str> = section.section.iter().map(String::as_str).collect();
    match capture_block(&text, &visit, &[key]).ok()? {
        PriorBlock::Present { entries } => entries.first().and_then(|entry| entry.value.clone()),
        PriorBlock::Absent => None,
    }
}

/// What a Steam block is, from the path it lives under.
fn steam_label(section: &PriorSection) -> &'static str {
    if section
        .section
        .iter()
        .any(|part| part == "CompatToolMapping")
    {
        "Proton pin"
    } else if section.section.iter().any(|part| part == "apps") {
        "Launch options"
    } else {
        "Steam setting"
    }
}

/// The app id a per-game block sits under, if any.
fn steam_app(section: &[String]) -> Option<&str> {
    let last = section.last()?;
    (!last.is_empty() && last.chars().all(|char| char.is_ascii_digit())).then_some(last.as_str())
}

/// Whether a sysctl key currently reads back, as a readable value.
fn sysctl_arrow(runner: &dyn CommandRunner, key: &str, target: &str) -> String {
    let path = Path::new("/proc/sys").join(key.replace('.', "/"));
    arrow(current(runner, &path).as_deref(), Some(target))
}

/// A file's current contents trimmed to one line, when they can be read.
fn current(runner: &dyn CommandRunner, path: &Path) -> Option<String> {
    runner
        .read_to_string(path)
        .ok()
        .map(|text| text.trim().to_owned())
}

/// `<current> → <target>`, or just `→ <target>` when the current value cannot
/// be read. "(empty)" stands for a value that is absent or blank.
fn arrow(current: Option<&str>, target: Option<&str>) -> String {
    let target = target.map_or_else(|| "(empty)".to_owned(), shown);
    match current {
        Some(current) => format!("{} → {target}", shown(current)),
        None => format!("→ {target}"),
    }
}

/// A value as it reads on a line: an empty string shows as "(empty)".
fn shown(value: &str) -> String {
    if value.is_empty() {
        "(empty)".to_owned()
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
#[path = "rollback_test.rs"]
mod rollback_test;
