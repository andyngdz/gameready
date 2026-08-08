//! One step that went wrong, laid out so the reader can stop reading early.

use std::fmt;

use console::style;
use gameready_core::improvement::{Remedy, Trouble};
use gameready_core::journal::RunId;

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::PROMPT;

/// A step whose ending needs explaining, written under its own name.
///
/// Three lines at most, always in the same order: what broke, what state the
/// machine is in now, and the one command that fixes it. The order is what
/// makes it skimmable. Someone who only reads the second line still learns the
/// thing they actually want to know.
pub(crate) struct WentWrong<'a> {
    mark: Mark,
    name: &'a str,
    trouble: &'a Trouble,
    run: &'a RunId,
}

impl<'a> WentWrong<'a> {
    pub(crate) const fn new(
        mark: Mark,
        name: &'a str,
        trouble: &'a Trouble,
        run: &'a RunId,
    ) -> Self {
        Self {
            mark,
            name,
            trouble,
            run,
        }
    }

    /// Writes the block into an open section.
    ///
    /// Takes the section rather than rendering itself, because these lines sit
    /// between the ordinary result rows and have to wrap to the same width.
    pub(crate) fn write<W: fmt::Write>(&self, section: &mut Section<'_, W>) -> fmt::Result {
        section.marked(self.mark, self.name)?;
        section.sub(&style(&self.trouble.broke).dim().to_string())?;
        section.sub(&style(&self.trouble.now).dim().to_string())?;

        match &self.trouble.fix {
            None => Ok(()),
            Some(Remedy::Rollback { lead }) => Self::offer(
                section,
                lead,
                &format!("gameready rollback --run {}", self.run),
            ),
            Some(Remedy::Yours { lead, command }) => Self::offer(section, lead, command),
        }
    }

    /// The sentence that frames a command, then the command on its own line so
    /// it can be copied without the sentence coming with it.
    fn offer<W: fmt::Write>(
        section: &mut Section<'_, W>,
        lead: &str,
        command: &str,
    ) -> fmt::Result {
        section.sub(&style(lead).dim().to_string())?;
        section.sub(&format!(
            "{} {}",
            style(PROMPT).green(),
            style(command).bold()
        ))
    }
}

#[cfg(test)]
#[path = "trouble_test.rs"]
mod trouble_test;
