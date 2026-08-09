//! Turning one finished step into the lines that report it.
//!
//! Every step is one row: mark, name, and what it did on a single line. Only a
//! failure breaks out of that shape, because it has three sentences to say
//! about the state the machine is in and a value column cannot hold them. A
//! skip stays a row and hands its one command back underneath, when a conflict
//! left the user something to run.

use std::fmt;

use console::style;
use gameready_core::improvement::{Outcome, Remedy, Trouble};
use gameready_core::journal::RunId;

use crate::cli::ui::layout::{Mark, Section};

/// One finished step, ready to write its own line (or block) into a section.
pub(crate) struct StepRow<'a> {
    pub mark: Mark,
    pub name: &'a str,
    pub outcome: &'a Outcome,
    pub column: usize,
    pub run: &'a RunId,
}

impl StepRow<'_> {
    /// Writes the step into an open section.
    pub(crate) fn write<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        // A conflict skip carries a trouble too, so a failure is told apart by
        // its kind rather than by whether a trouble exists.
        match (
            matches!(self.outcome, Outcome::Failed { .. }),
            self.outcome.trouble(),
        ) {
            (true, Some(trouble)) => self.failure(s, &trouble),
            _ => self.result(s),
        }
    }

    /// A step that landed or stood down, on one line, with the one command a
    /// conflict leaves the user printed under it.
    fn result<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        match self.outcome.detail() {
            Some(detail) => s.row(self.mark, self.name, &detail, self.column)?,
            None => s.marked(self.mark, self.name)?,
        }
        match self.own_command() {
            Some(command) => s.sub(&copyable(&command)),
            None => Ok(()),
        }
    }

    /// A step that broke: what broke, the state it left, and the undo to retry
    /// when the undo is what failed.
    fn failure<W: fmt::Write>(&self, s: &mut Section<'_, W>, trouble: &Trouble) -> fmt::Result {
        s.marked(self.mark, self.name)?;
        s.sub(&style(&trouble.broke).dim().to_string())?;
        s.sub(&style(&trouble.now).dim().to_string())?;
        match &trouble.fix {
            None => Ok(()),
            Some(Remedy::Rollback { lead }) => Self::offer(s, lead, &undo(self.run)),
            Some(Remedy::Yours { lead, command }) => Self::offer(s, lead, command),
        }
    }

    /// A framing sentence, then the command on its own line so it copies clean.
    fn offer<W: fmt::Write>(s: &mut Section<'_, W>, lead: &str, command: &str) -> fmt::Result {
        s.sub(&style(lead).dim().to_string())?;
        s.sub(&copyable(command))
    }

    /// The one command a skip leaves the user, if a conflict handed one back.
    fn own_command(&self) -> Option<String> {
        match self.outcome.trouble()?.fix? {
            Remedy::Yours { command, .. } => Some(command),
            Remedy::Rollback { .. } => None,
        }
    }
}

/// A command on its own line, bold so it stands out as the one line worth
/// copying.
pub(crate) fn copyable(command: &str) -> String {
    style(command).bold().to_string()
}

/// This run's own undo, spelled with the run id only the report holds.
pub(crate) fn undo(run: &RunId) -> String {
    format!("gameready rollback --run {run}")
}

#[cfg(test)]
#[path = "rows_test.rs"]
mod rows_test;
