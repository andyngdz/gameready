//! Asking whether to keep the CPU governor pinned across reboots.

use std::fmt;

use anyhow::Result;
use inquire::Select;

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
        "Keep the CPU on performance after you reboot?",
        vec![Persistence::ThisBoot, Persistence::KeepIt],
    )
    .with_help_message(
        "nothing else here raises the clocks, so I can pin it; it runs hot on a laptop, and \
         `gameready rollback` undoes it now either way",
    )
    .prompt_skippable()?;

    Ok(matches!(answer, Some(Persistence::KeepIt)))
}

#[cfg(test)]
#[path = "governor_test.rs"]
mod governor_test;
