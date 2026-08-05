//! What probing found, and how a step ended.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::improvement::domain::identity::ImprovementId;
use crate::improvement::domain::verify::Verification;
use crate::journal::Change;

/// What probing a step found, before anything is changed. Probing must not
/// mutate: the executor probes every selected step first so it can show a
/// complete plan and fail cheaply on preconditions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Probe {
    /// Not applied, and this system can take it.
    Applicable,

    /// Already in the desired state. `evidence` is what was read to decide,
    /// so the summary can say why rather than just "skipped".
    AlreadyApplied { evidence: String },

    /// This system cannot take it, and no amount of installing will change
    /// that: kernel too old, package absent from every configured repo.
    NotApplicable { reason: String },

    /// Something else owns this setting and would fight us over it.
    Conflict { with: String, detail: String },

    /// Probing itself failed. Treated as a skip, never as permission to apply,
    /// because a step that cannot read the current state cannot restore it.
    Unknown { reason: String },
}

/// How a step ended. One of these is recorded per step per run and is what the
/// summary screen and `--json` output are built from.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum Outcome {
    /// Applied and verified. `changes` is the undo record; `verification` is
    /// the proof the change took effect.
    Applied {
        changes: Vec<Change>,
        verification: Verification,
        took: Duration,
    },

    /// Found already in the desired state, nothing done.
    AlreadyApplied { evidence: String },

    /// Deliberately not run. Distinct from `NotApplicable`: this system could
    /// have taken it.
    Skipped { reason: SkipReason },

    /// This system cannot take it.
    NotApplicable { reason: String },

    /// Failed. `rolled_back` says whether the partial change was undone, which
    /// is the difference between a clean failure and one needing attention.
    Failed {
        error: String,
        rolled_back: RollbackStatus,
    },
}

impl Outcome {
    /// Whether this outcome should make the whole run exit non-zero.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Short word for the right-hand column of the progress list.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Applied { .. } => "applied",
            Self::AlreadyApplied { .. } => "already set",
            Self::Skipped { .. } => "skipped",
            Self::NotApplicable { .. } => "not applicable",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Why a step that could have run did not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum SkipReason {
    /// The user declined it on the plan screen.
    UserDeclined,

    /// Something else owns the setting.
    Conflict { with: String },

    /// A step this one declared in `requires()` failed, so running this would
    /// build on a state that does not exist.
    DependencyFailed { on: ImprovementId },

    /// A prerequisite could not be installed, so the step cannot run.
    MissingDependency { name: String, detail: String },

    /// Steam holds its config in memory and rewrites the file on exit, so
    /// writing it now would be silently discarded. Queued as pending.
    SteamRunning,

    /// `--dry-run`: the plan was computed, nothing was touched.
    DryRun,
}

/// Whether a failed step left anything behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RollbackStatus {
    /// The step failed before changing anything, so there was nothing to undo.
    NotAttempted,

    /// The partial change was undone; the system is as it was.
    Succeeded,

    /// The undo itself failed. This is the one state that needs the user to
    /// look at something, so it carries the detail and the journal keeps the
    /// records for a manual `gameready rollback`.
    Failed { detail: String },
}
