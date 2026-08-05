//! What exercising one step end to end proved.

use serde::{Deserialize, Serialize};

use crate::improvement::ImprovementId;

/// One step's result from a selftest run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepSelftest {
    pub step: ImprovementId,
    pub result: SelftestResult,
}

impl StepSelftest {
    /// Whether this should make the whole selftest exit non-zero.
    ///
    /// A skip is not a failure: a machine that cannot take a step has told us
    /// something true about itself, not about the step.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(
            self.result,
            SelftestResult::ProbeFailed { .. } | SelftestResult::Failed { .. }
        )
    }
}

/// How far a step got through apply, verify, rollback, and verify-reverted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum SelftestResult {
    /// Not exercised, because probing said this machine cannot take it now.
    Skipped { reason: String },

    /// Probing itself failed, which is a real fault: a step that cannot read
    /// the current state cannot restore it either.
    ProbeFailed { error: String },

    /// Every phase held.
    Passed { reverted: RevertCheck },

    /// One phase did not hold, named so the output points at the phase rather
    /// than at the step as a whole.
    Failed { phase: Phase, detail: String },
}

/// Which phase of the cycle a failure landed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Apply,
    Verify,
    Rollback,
    Reverted,
}

impl Phase {
    /// The phase name as it reads on the selftest line.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Apply => "apply",
            Self::Verify => "verify",
            Self::Rollback => "rollback",
            Self::Reverted => "reverted",
        }
    }
}

/// Whether "the change is gone again" was something this step could be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevertCheck {
    /// The change was read back as gone.
    Confirmed,

    /// There was nothing to read back. Every change the step made was a package
    /// install, whose documented undo is a report rather than a removal, so
    /// demanding the change disappear would fail a step that behaved correctly.
    NotApplicable,
}

impl RevertCheck {
    /// How the check reads on the selftest line.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Confirmed => "the change is gone again",
            Self::NotApplicable => "nothing to revert",
        }
    }
}

#[cfg(test)]
#[path = "selftest_test.rs"]
mod selftest_test;
