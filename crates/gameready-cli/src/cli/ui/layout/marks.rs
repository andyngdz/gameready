//! The one place a gutter mark is chosen.

use console::style;
use gameready_core::improvement::{OutcomeKind, Probe};

/// Every mark the CLI draws in the two-column gutter.
///
/// A vocabulary rather than a handful of loose glyphs. The same thing has to
/// look the same in the live region and in the summary printed a second later,
/// and two call sites each reaching for their own tick is exactly how those
/// two drifted apart before this existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// Changed, and verified afterwards.
    Applied,

    /// Found already correct, and left alone. Deliberately not a tick: a tick
    /// beside a step the run never touched tells the user their machine
    /// changed, and the next thing they do with that belief is roll back
    /// something that was never applied.
    AlreadySet,

    /// Failed, with whatever it had done undone.
    Failed,

    /// Declined, or cannot run on this machine.
    Skipped,

    /// Worth reading, but nothing failed.
    Warning,

    /// The run looked at a step a second time.
    Recheck,

    /// One entry in a list of things the run is about to act on.
    Chosen,
}

impl Mark {
    /// How a step that has finished is marked.
    #[must_use]
    pub(crate) const fn of(kind: OutcomeKind) -> Self {
        match kind {
            OutcomeKind::Applied => Self::Applied,
            OutcomeKind::AlreadySet => Self::AlreadySet,
            OutcomeKind::Failed => Self::Failed,
            OutcomeKind::Skipped | OutcomeKind::NotApplicable => Self::Skipped,
        }
    }

    /// The mark the doctor screen draws for what a probe found. `doctor` never
    /// changes anything, so an applicable step reads as a tick meaning "would
    /// apply", not "applied".
    #[must_use]
    pub(crate) fn for_probe(probe: &Probe) -> Self {
        match probe {
            Probe::Applicable => Self::Applied,
            Probe::AlreadyApplied { .. } => Self::AlreadySet,
            Probe::Conflict { .. } => Self::Warning,
            Probe::NotApplicable { .. } | Probe::Unknown { .. } => Self::Skipped,
        }
    }

    /// The glyph, coloured, exactly one column wide.
    ///
    /// One column is a contract, not an accident: every marked line prefixes
    /// the glyph with a fixed indent and a trailing space, so a wider glyph
    /// would push one row's text out of line with the rest.
    #[must_use]
    pub(crate) fn glyph(self) -> String {
        match self {
            Self::Applied => style("\u{2713}").green().to_string(),
            Self::AlreadySet => style("\u{2022}").green().dim().to_string(),
            Self::Failed => style("\u{2718}").red().bold().to_string(),
            Self::Skipped => style("-").dim().to_string(),
            Self::Warning => style("!").yellow().to_string(),
            Self::Recheck => style("\u{21bb}").blue().to_string(),
            Self::Chosen => style("*").green().to_string(),
        }
    }
}

#[cfg(test)]
#[path = "marks_test.rs"]
mod marks_test;
