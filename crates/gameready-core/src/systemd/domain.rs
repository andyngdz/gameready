//! What a unit is doing.

use serde::{Deserialize, Serialize};

/// Where a unit stands on this machine.
///
/// Four states rather than a pair of booleans, because the question callers ask
/// is "will this thing run", and `enabled: false, active: true` reads as no
/// under a boolean pair while meaning yes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "unit_state", rename_all = "snake_case")]
pub enum UnitState {
    /// No unit file with that name. The package providing it is not installed.
    Absent,

    /// Installed, not running, and not set to start.
    Dormant,

    /// Set to start at boot but not running yet, usually because the machine
    /// has not rebooted since it was enabled.
    EnabledNotStarted,

    /// Running now.
    Running,
}

impl UnitState {
    /// Whether this unit will act on the system, now or after the next boot.
    ///
    /// The question conflict detection asks. A dormant unit is installed but
    /// inert, and warning about it would be noise.
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::EnabledNotStarted | Self::Running)
    }

    /// How the state reads on the summary screen.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Absent => "not installed",
            Self::Dormant => "installed but not running",
            Self::EnabledNotStarted => "enabled, starts at next boot",
            Self::Running => "running",
        }
    }
}

#[cfg(test)]
#[path = "domain_test.rs"]
mod domain_test;
