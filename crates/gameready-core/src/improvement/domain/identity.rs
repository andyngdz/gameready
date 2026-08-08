//! Stable identity and metadata shared by every improvement.

use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::improvement::errors::ImprovementIdError;

/// The journal key for an improvement. Once shipped it must never change:
/// a stored run references steps by this string, so renaming one orphans
/// every undo record that names it.
///
/// Shape is dot-separated kebab-case segments, `core.sysctl.max-map-count`.
/// Built-in steps carry a `&'static str`; steps expanded from a game profile
/// own their string, hence the [`Cow`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ImprovementId(Cow<'static, str>);

impl ImprovementId {
    /// Wraps a literal authored in this crate. Not validated here, because a
    /// literal cannot be corrected at runtime; `registry` has a test that walks
    /// every registered step and asserts the shape instead.
    #[must_use]
    pub const fn from_static(id: &'static str) -> Self {
        Self(Cow::Borrowed(id))
    }

    /// Validates an id built at runtime, such as one derived from a game
    /// profile name. Rejects anything that would not round-trip through the
    /// journal or read cleanly on the plan screen.
    pub fn parse(id: impl Into<String>) -> Result<Self, ImprovementIdError> {
        let id = id.into();
        validate(&id)?;
        Ok(Self(Cow::Owned(id)))
    }

    /// The id as written in the journal and on screen.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// First segment, used to group steps on the plan and summary screens.
    #[must_use]
    pub fn namespace(&self) -> &str {
        self.0.split('.').next().unwrap_or(&self.0)
    }
}

fn validate(id: &str) -> Result<(), ImprovementIdError> {
    if id.is_empty() {
        return Err(ImprovementIdError::Empty);
    }

    for segment in id.split('.') {
        if segment.is_empty() {
            return Err(ImprovementIdError::EmptySegment { id: id.to_owned() });
        }
        let shaped = segment
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !shaped {
            return Err(ImprovementIdError::Malformed {
                id: id.to_owned(),
                segment: segment.to_owned(),
            });
        }
    }

    Ok(())
}

impl fmt::Display for ImprovementId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether a step needs root to apply. Drives the pre-flight sudo priming and
/// the "we will ask for your password" warning, so a step that lies here makes
/// the run prompt at an unexpected moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Privilege {
    /// Runs entirely as the invoking user: files under `$HOME`, Steam config.
    User,
    /// Needs root for at least one command.
    Root,
}

/// Coarse subject area, used for `--only <tag>` and for grouping output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tag {
    Cpu,
    Gpu,
    Io,
    Memory,
    Scheduler,
    Overlay,
    Wine,
    Steam,
}

impl Tag {
    /// What this area is called on a screen a user reads.
    ///
    /// Not the variant name lowercased: "io" and "scheduler" are how the code
    /// talks about the machine, and a plan that says it will change "io" has
    /// told the user nothing.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cpu => "the CPU",
            Self::Gpu => "graphics",
            Self::Io => "disks",
            Self::Memory => "memory",
            Self::Scheduler => "the CPU scheduler",
            Self::Overlay => "the overlay",
            Self::Wine => "Wine and Proton",
            Self::Steam => "Steam",
        }
    }
}

#[cfg(test)]
#[path = "identity_test.rs"]
mod identity_test;
