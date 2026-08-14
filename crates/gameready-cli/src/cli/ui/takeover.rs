//! Asking whether to take a setting over from whoever owns it.
//!
//! A step that probed into a conflict is not a failure: something else is
//! running the setting, and the question is whether to hand it to gameready's
//! pick. Saying no is the safe answer, and an escaped prompt is a no.

use std::fmt;

use anyhow::Result;
use gameready_core::improvement::ImprovementId;
use gameready_core::run::Contested;

use crate::cli::ui::theme;

/// The two answers to the takeover question.
#[derive(Clone, Copy)]
enum Takeover {
    /// Stop the owner and run gameready's pick.
    TakeIt,
    /// Leave the owner alone.
    LeaveIt,
}

impl fmt::Display for Takeover {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TakeIt => "Take it over",
            Self::LeaveIt => "Leave it running",
        })
    }
}

/// Asks whether to take one contested setting over.
///
/// The question names the step's own words, and the body names the owner, the
/// step's gains line (which carries its honest limits), and the one cost that
/// has to be said out loud: the owner stops while the take-over runs.
///
/// Defaults to leaving the owner alone, and an escaped prompt is a no: taking
/// over something the user never set up is the one answer this run must not
/// guess at.
pub fn choose_takeover(contested: &Contested) -> Result<bool> {
    let question = format!("{}?", contested.step.name());
    let why = format!(
        "{} is already doing this. Not a promise of more FPS. {} {} stops while this takes \
         over.",
        contested.with,
        contested.step.gains().unwrap_or_default(),
        contested.with,
    );
    let either_way = format!(
        "Either way: gameready rollback puts {} back now, no reboot.",
        contested.with
    );

    let answer = theme::Asked::new(&question, &why, &either_way)
        .one_of(vec![Takeover::TakeIt, Takeover::LeaveIt])
        .prompt_skippable()?;

    Ok(matches!(answer, Some(Takeover::TakeIt)))
}

/// Asks about every contested step and returns the ids to take over.
///
/// The driver behind both `init`'s question pass and `apply`, so the two
/// cannot drift apart on which answer moves a step into the run.
pub fn ask_takeovers(contested: &[Contested]) -> Result<Vec<ImprovementId>> {
    let mut agreed = Vec::new();
    for entry in contested {
        if choose_takeover(entry)? {
            agreed.push(entry.step.id());
        }
    }
    Ok(agreed)
}

#[cfg(test)]
#[path = "takeover_test.rs"]
mod takeover_test;
