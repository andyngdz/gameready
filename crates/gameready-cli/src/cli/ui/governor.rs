//! Asking whether to keep the CPU governor pinned across reboots.

use std::fmt;

use anyhow::Result;
use inquire::Select;

use crate::cli::ui::theme;

/// The question.
const QUESTION: &str = "Keep the CPU on performance after you reboot?";

/// Why this run is the one that gets asked, and what it costs to say yes.
const WHY_ASKED: &str =
    "Nothing else on this machine raises the clocks, so I can pin performance. \
                         A pinned governor runs hot and drains a laptop until you undo it.";

/// The reassurance under both answers.
const EITHER_WAY: &str = "Either way: gameready rollback undoes it now.";

/// The two answers to the governor-persistence question.
#[derive(Clone, Copy)]
enum Persistence {
    /// The default: the live change lasts until the next boot, no file written.
    ThisBoot,
    /// A udev rule re-pins it every boot, until a rollback removes it.
    KeepIt,
}

impl fmt::Display for Persistence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ThisBoot => "Just this boot, reboot puts it back",
            Self::KeepIt => "Keep it, until I roll back",
        })
    }
}

/// Asks whether to keep the CPU governor on performance across reboots.
///
/// Only reached when the run will actually pin the governor, because nothing
/// else on the machine raises the clocks. Defaults to this boot only, and an
/// escaped prompt keeps that safe answer: a pinned governor runs hot and drains
/// a laptop until it is undone, and `gameready rollback` undoes it now either
/// way.
pub fn choose_governor_persistence() -> Result<bool> {
    let answer = Select::new(
        &theme::asked(QUESTION, WHY_ASKED),
        vec![Persistence::ThisBoot, Persistence::KeepIt],
    )
    .with_render_config(theme::Prompts::choices())
    .with_help_message(EITHER_WAY)
    .prompt_skippable()?;

    Ok(matches!(answer, Some(Persistence::KeepIt)))
}

#[cfg(test)]
#[path = "governor_test.rs"]
mod governor_test;
