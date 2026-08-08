//! What the run is about to do, shown before it does any of it.

use std::fmt;

use console::style;
use gameready_core::run::Mode;
use gameready_core::steam::GameSetup;

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::questions::Answers;

/// The agreed plan, printed before the first change.
pub struct InitPlan<'a> {
    found: &'a [GameSetup],
    answers: &'a Answers,
    mode: Mode,
}

impl<'a> InitPlan<'a> {
    #[must_use]
    pub const fn new(found: &'a [GameSetup], answers: &'a Answers, mode: Mode) -> Self {
        Self {
            found,
            answers,
            mode,
        }
    }
}

impl fmt::Display for InitPlan<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.found.is_empty() {
            writeln!(f, "\n  {}", style("No games found.").dim())?;
            return Ok(());
        }

        let mut s = Section::new(f);
        s.title("Game selected:")?;
        for setup in &self.answers.selected {
            s.marked(Mark::Chosen, &setup.game.name)?;
        }
        if !self.mode.mutates() {
            s.indented(&style("Dry run: nothing will change.").dim().to_string())?;
        }
        s.end()
    }
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
