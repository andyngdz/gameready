//! Proof that a change took effect.
//!
//! The executor calls `verify()` immediately after `apply()`. If any check
//! fails the step is not reported as applied: it is rolled back from its own
//! journal records and reported as failed. That is what makes "every step is
//! tested" a property of the engine rather than a promise in a README.
//!
//! What this proves is narrow and worth stating plainly: that the system now
//! reads back the value we wrote. It does not prove a frame rate improved.
//! Benchmarking is a separate concern and deliberately not modelled here.

use serde::{Deserialize, Serialize};

/// One readback: what was inspected, what it should say, what it actually says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Check {
    /// What was inspected, in the terms a user would recognise.
    /// "runtime vm.max_map_count"
    pub what: String,

    /// The value the step intended to produce.
    pub expected: String,

    /// The value actually read back.
    pub actual: String,

    /// Whether `actual` satisfies `expected`. Kept as a field rather than
    /// derived by comparing the two strings, because some checks pass on a
    /// range or a substring rather than equality.
    pub pass: bool,
}

impl Check {
    /// A check that passes when the two values match exactly.
    #[must_use]
    pub fn equals(
        what: impl Into<String>,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        let expected = expected.into();
        let actual = actual.into();
        Self {
            what: what.into(),
            pass: expected == actual,
            expected,
            actual,
        }
    }
}

/// The full set of readbacks for one step.
///
/// An empty `Verification` is a bug, not a valid result: a step that cannot
/// prove its own effect must not report success. The registry test asserts
/// every step produces at least one check.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Verification {
    /// Every readback performed, in the order performed.
    pub checks: Vec<Check>,
}

impl Verification {
    /// An empty set, to be filled with [`Verification::check`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one readback.
    #[must_use]
    pub fn check(mut self, check: Check) -> Self {
        self.checks.push(check);
        self
    }

    /// Whether every check passed. An empty set returns `false`: nothing was
    /// proven, so nothing may be claimed.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|check| check.pass)
    }

    /// How many checks did not pass.
    #[must_use]
    pub fn failed_count(&self) -> usize {
        self.checks.iter().filter(|check| !check.pass).count()
    }

    /// Total checks performed.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.checks.len()
    }
}

#[cfg(test)]
#[path = "verify_test.rs"]
mod verify_test;
