//! The one place a colour is chosen.
//!
//! A vocabulary rather than loose hex literals, for the same reason the CLI
//! keeps its gutter marks in one enum: the icon and the dots beside the rows
//! have to agree on what green means, and two call sites each picking their own
//! shade is how they stop agreeing.

use gameready_core::improvement::ProbeStatus;

/// A colour role, resolved to pixels by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    /// The controller at rest, on a bar dark enough to need a light icon.
    Light,

    /// The controller at rest, on a bar light enough to need a dark icon.
    Dark,

    /// In place: a tuning that is applied, or a configured game running now.
    Live,

    /// Would apply, and has not yet.
    Pending,

    /// Something else owns this, or it could not be read.
    Alert,

    /// Ruled out. Nothing will happen here.
    Muted,
}

impl Ink {
    /// The environment variable a user sets to pick the resting colour.
    pub const VARIABLE: &'static str = "GAMEREADY_TRAY_ICON";

    /// What the user writes in [`Ink::VARIABLE`] to get a dark icon.
    const DARK: &'static str = "dark";

    /// The colour the controller rests in when no configured game is running.
    ///
    /// A bar's background is not readable across desktops, so this is the
    /// user's call rather than a guess that would leave the icon invisible on
    /// half of them. Light by default, because most bars are dark. Anything
    /// unrecognised is the default too: a typo should leave a visible icon,
    /// not refuse to start.
    #[must_use]
    pub fn resting(chosen: Option<&str>) -> Self {
        if chosen.map(str::trim) == Some(Self::DARK) {
            Self::Dark
        } else {
            Self::Light
        }
    }

    /// The dot drawn beside a row in this state.
    #[must_use]
    pub const fn for_status(status: ProbeStatus) -> Self {
        match status {
            // An update available is still installed, so it wears the live
            // green; its note line is what tells the two apart.
            ProbeStatus::Set | ProbeStatus::UpdateAvailable => Self::Live,
            ProbeStatus::Ready => Self::Pending,
            ProbeStatus::Attention => Self::Alert,
            ProbeStatus::Inactive => Self::Muted,
        }
    }

    /// This role in 8-bit red, green, and blue.
    ///
    /// Mid-weight rather than saturated: these sit at 16 pixels next to text,
    /// where a full-strength red reads as an error the user has to act on and
    /// a conflict is not one.
    #[must_use]
    pub const fn rgb(self) -> (u8, u8, u8) {
        match self {
            Self::Light => (0xE8, 0xEA, 0xED),
            Self::Dark => (0x20, 0x23, 0x24),
            Self::Live => (0x3F, 0xB9, 0x50),
            Self::Pending => (0xE3, 0xA5, 0x21),
            Self::Alert => (0xD9, 0x53, 0x4F),
            Self::Muted => (0x8A, 0x8F, 0x98),
        }
    }
}

#[cfg(test)]
#[path = "ink_test.rs"]
mod ink_test;
