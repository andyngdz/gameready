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
    ///
    /// `detail` names the owner itself, because it is the sentence the user
    /// reads. `yours` is the one command that would hand the setting back, and
    /// only the step knows it: disabling a systemd unit and unloading a
    /// scheduler somebody else started are not the same instruction, and for
    /// some owners there is no single command at all.
    Conflict {
        with: String,
        detail: String,
        yours: Option<String>,
    },

    /// Probing itself failed. Treated as a skip, never as permission to apply,
    /// because a step that cannot read the current state cannot restore it.
    Unknown { reason: String },
}

impl Probe {
    /// What was found, in the words shown to the user.
    ///
    /// Lives here rather than in the CLI for the same reason as
    /// [`Outcome::detail`]: what there is to say about a probe result is a
    /// property of the result. The CLI decides the layout, this decides the
    /// words, and `doctor` and the plan screen cannot drift apart.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Applicable => "I would apply this".to_owned(),
            Self::AlreadyApplied { evidence } => format!("already set ({evidence})"),
            Self::NotApplicable { reason } => format!("not applicable ({reason})"),
            Self::Conflict { detail, .. } => detail.clone(),
            Self::Unknown { reason } => format!("I could not tell ({reason})"),
        }
    }
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

    /// The one-line explanation shown under a step on the summary screen.
    ///
    /// Lives here rather than in the CLI because what there is to say about an
    /// outcome is a property of the outcome. The CLI decides the colour and the
    /// mark; this decides the words.
    #[must_use]
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Applied { verification, .. } => Some(format!(
                "I verified {} of {} checks",
                verification.total_count() - verification.failed_count(),
                verification.total_count(),
            )),
            Self::AlreadyApplied { evidence } => Some(evidence.clone()),
            Self::NotApplicable { reason } => Some(reason.clone()),
            Self::Skipped { reason } => Some(reason.describe()),
            Self::Failed { error, rolled_back } => {
                Some(format!("{error}; {}", rolled_back.describe()))
            }
        }
    }

    /// Short word for the right-hand column of the progress list.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        self.kind().label()
    }

    /// Which broad bucket this outcome falls into.
    #[must_use]
    pub const fn kind(&self) -> OutcomeKind {
        match self {
            Self::Applied { .. } => OutcomeKind::Applied,
            Self::AlreadyApplied { .. } => OutcomeKind::AlreadySet,
            Self::Skipped { .. } => OutcomeKind::Skipped,
            Self::NotApplicable { .. } => OutcomeKind::NotApplicable,
            Self::Failed { .. } => OutcomeKind::Failed,
        }
    }
}

/// The broad bucket an outcome falls into, without carrying the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutcomeKind {
    Applied,
    AlreadySet,
    Skipped,
    NotApplicable,
    Failed,
}

impl OutcomeKind {
    /// Short word for the right-hand column of the progress list.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::AlreadySet => "already set",
            Self::Skipped => "skipped",
            Self::NotApplicable => "not applicable",
            Self::Failed => "failed",
        }
    }
}

/// Why a step that could have run did not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum SkipReason {
    /// The user declined it on the plan screen.
    UserDeclined,

    /// Something else owns the setting. Carries what the probe found whole,
    /// because the summary has to say what owns it, why that settles it, and
    /// what the user could run about it.
    Conflict {
        with: String,
        detail: String,
        yours: Option<String>,
    },

    /// The probe could not read the current state. Never a reason to apply: a
    /// step that cannot tell what is there cannot put it back.
    ///
    /// Carries `detail` rather than `reason`, which is the tag this enum
    /// serializes its variant name into.
    CouldNotTell { detail: String },

    /// A step this one declared in `requires()` failed, so running this would
    /// build on a state that does not exist.
    DependencyFailed { on: ImprovementId },

    /// A prerequisite could not be installed, so the step cannot run.
    MissingDependency { name: String, detail: String },

    /// `--dry-run`: the plan was computed, nothing was touched.
    DryRun,
}

impl SkipReason {
    /// The reason, in the words shown to a user.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::UserDeclined => "you declined it".to_owned(),
            Self::Conflict { with, .. } => format!("I left it to {with}, which is running"),
            Self::CouldNotTell { detail } => format!("I could not tell ({detail})"),
            Self::DependencyFailed { on } => format!("{on} failed, so I did not build on it"),
            Self::MissingDependency { name, detail } => {
                format!("I could not get {name}: {detail}")
            }
            Self::DryRun => "dry run".to_owned(),
        }
    }
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

impl RollbackStatus {
    /// What happened to the partial change, in the words shown to a user.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::NotAttempted => "nothing had changed yet".to_owned(),
            Self::Succeeded => "I undid the partial change".to_owned(),
            Self::Failed { detail } => {
                format!("I could not finish undoing it ({detail}); run `gameready rollback`")
            }
        }
    }
}
