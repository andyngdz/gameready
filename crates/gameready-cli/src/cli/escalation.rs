//! The one password prompt a run owes, and when it comes due.

use anyhow::Result;

use crate::cli::args::Effect;

/// Whether this run has a password to ask for, and how to ask.
///
/// Held rather than called when the command starts. The prompt is the last
/// question a run asks, so it comes after the install question rather than
/// before the command does anything: a user is never asked to decide something
/// once the alternative has already been taken away from them.
#[derive(Clone, Copy)]
pub enum Escalation<'a> {
    /// Fill the credential cache through this before the first change.
    Ask(&'a dyn Fn() -> Result<()>),

    /// Nothing to ask for. A dry run changes nothing, and asking for a password
    /// to change nothing is a question with no purpose; on a machine whose sudo
    /// cannot cache, it is a question that stops the preview working at all.
    NotNeeded,
}

impl<'a> Escalation<'a> {
    /// What a command with this effect owes.
    ///
    /// The one place the decision is made, so `rollback` and `selftest` cannot
    /// drift from `init` and `apply`. `Command::mode` answers `Mode::Apply` for
    /// commands that have no `--dry-run` of their own, which makes it useless
    /// as a gate here; `Command::effect` covers all four.
    #[must_use]
    pub const fn for_effect(effect: Effect, prompt: &'a dyn Fn() -> Result<()>) -> Self {
        match effect {
            Effect::Mutates => Self::Ask(prompt),
            Effect::Reads => Self::NotNeeded,
        }
    }

    /// Asks now, if there is anything to ask.
    ///
    /// Every privileged command runs with `sudo -n`, which refuses to prompt.
    /// Without this the first one fails against a cold cache, and so does the
    /// rollback that tries to clean up after it.
    pub fn ask(&self) -> Result<()> {
        match self {
            Self::Ask(prompt) => prompt(),
            Self::NotNeeded => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "escalation_test.rs"]
mod escalation_test;
