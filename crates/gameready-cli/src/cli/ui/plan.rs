//! What the run is about to do, shown before it does any of it.

use std::fmt;

use gameready_core::run::Mode;
use gameready_core::steam::GameSetup;

use crate::cli::ui::questions::Answers;

/// The agreed plan, printed before the first change.
///
/// Every line here is something the user has already answered a question about.
/// It exists so they can see the answers together, in the order they will
/// happen, rather than reconstructing them from what scrolled past.
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
        writeln!(f, "\nGames found")?;
        if self.found.is_empty() {
            writeln!(f, "{}", crate::cli::ui::NOTHING)?;
        }
        for setup in self.found {
            let chosen = self
                .answers
                .selected
                .iter()
                .any(|picked| picked.game.app_id == setup.game.app_id);
            writeln!(
                f,
                "  {} {:<30} {:>8}  {}",
                if chosen { "*" } else { " " },
                setup.game.name,
                setup.game.app_id,
                if setup.has_profile() {
                    "has a profile"
                } else {
                    "no profile yet, core tuning still applies"
                },
            )?;
        }

        if !self.mode.mutates() {
            writeln!(f, "\n  Dry run: nothing below will actually change.")?;
        }

        if self.answers.closes_steam() {
            writeln!(
                f,
                "\n  Steam will be closed to write launch options for {} game(s).",
                self.answers.targets.len()
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
