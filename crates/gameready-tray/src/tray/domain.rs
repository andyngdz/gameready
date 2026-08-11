//! What the panel shows, with nothing in it that knows about D-Bus.
//!
//! Every type here is plain data so the whole screen can be built and asserted
//! on without a session bus, which is the only way any of this is testable.

use std::fmt;

use gameready_core::doctor::StepFinding;
use gameready_core::games::AppId;
use gameready_core::improvement::ProbeStatus;

/// One tuning, as one line of the menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The tuning's short name, on the left.
    pub label: String,

    /// Which bucket the probe fell in, which decides the dot's colour.
    pub status: ProbeStatus,

    /// A second, read-only line under the row, drawn only when a row earns one.
    pub note: Option<String>,
}

impl Row {
    /// Whether this tuning is in place right now.
    ///
    /// Counted for the hover line, which is the only thing a user sees without
    /// opening the menu, so it says how many of the tunings hold rather than
    /// how many rows there are.
    #[must_use]
    pub fn is_set(&self) -> bool {
        self.status == ProbeStatus::Set
    }
}

impl Row {
    /// Builds one menu row from a step's own bar label and what probing found.
    ///
    /// The label comes from the step rather than from the finding, because
    /// `StepFinding` carries the terminal identifier the doctor screen wants
    /// and a panel menu wants the step's [`Improvement::bar_name`].
    #[must_use]
    pub fn new(bar_name: &str, finding: &StepFinding) -> Self {
        let status = finding.status();
        Self {
            label: bar_name.to_owned(),
            status,
            // The one note a panel row earns is a version hint; every other
            // sentence a finding could add belongs to `gameready doctor`.
            note: (status == ProbeStatus::UpdateAvailable).then(|| finding.note()),
        }
    }
}

impl fmt::Display for Row {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The name alone: the coloured dot carries the state, and the detail a
        // sentence would add belongs in `gameready doctor`, which has the width
        // for it.
        write!(formatter, "{}", self.label)
    }
}

/// Whether a game gameready set up is running right now.
///
/// Drives the icon's colour, and nothing else. A game nobody configured is
/// [`Activity::Idle`] as far as this tray is concerned, because a green icon
/// claims gameready's tuning is live and for that game it is not.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Activity {
    /// No configured game is registered with gamemode.
    #[default]
    Idle,

    /// A game with a gameready profile is running.
    Playing {
        /// The profile's name, which the submenu is titled with.
        game: String,

        /// Which game, for probing the two tunings that belong to it.
        app_id: AppId,

        /// What gameready set for it, empty when there is nothing to say.
        rows: Vec<Row>,
    },
}

/// Everything one sweep learned, or why it learned nothing.
///
/// A state rather than a bare list, because "every tuning was ruled out" and
/// "the machine could not be read" both produce no rows, and one of those is
/// a working machine while the other is a broken tray.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Snapshot {
    /// Every core tuning, in registry order.
    Ready {
        /// One row per step.
        rows: Vec<Row>,
    },

    /// The machine could not be read at all, so no row would be honest.
    Unreadable {
        /// What failed, in the words the user sees.
        reason: String,
    },
}

impl fmt::Display for Snapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { reason } => {
                writeln!(formatter, "Could not read this machine: {reason}")
            }
            Self::Ready { rows } => {
                for row in rows {
                    writeln!(formatter, "{row}")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
#[path = "domain_test.rs"]
mod domain_test;
