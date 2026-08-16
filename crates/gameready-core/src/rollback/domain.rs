//! What undoing a run involves, and how it went.

use crate::improvement::{ImprovementId, Privilege};
use crate::journal::{RunId, Undo};

/// One recorded change, paired with the step that made it.
///
/// The step is kept so a rollback can be reported per improvement rather than
/// as an undifferentiated list of file and sysctl operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedUndo {
    /// The step whose change this reverses.
    pub step: ImprovementId,

    /// Position in the original run. Undo runs in descending order.
    pub seq: u64,

    /// The operation itself.
    pub undo: Undo,
}

/// Everything a rollback would do, in the order it would do it.
///
/// Built without touching the system, so `rollback --dry-run` shows exactly
/// what would happen and the ordering is testable on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackPlan {
    /// The run being undone.
    pub run: RunId,

    /// Operations in reverse order of the original run.
    pub undos: Vec<PlannedUndo>,
}

impl RollbackPlan {
    /// Whether there is anything to do.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.undos.is_empty()
    }

    /// Whether undoing this run needs root.
    ///
    /// A run that only wrote in the user's own home is undone as the user, and
    /// asking for a password to delete a file they own teaches them to type it
    /// without reading what asked.
    #[must_use]
    pub fn needs_root(&self) -> bool {
        self.undos
            .iter()
            .any(|planned| planned.undo.privilege() == Privilege::Root)
    }
}

/// How one undo operation ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoOutcome {
    /// The change was reversed.
    Reverted { detail: String },

    /// There was nothing left to reverse, which is not a failure: rollback has
    /// to be safe to re-run after a partial undo.
    AlreadyGone,

    /// Deliberately not done. Removing packages is the usual reason.
    Left { reason: String },

    /// Refused, because doing it would destroy something the user changed
    /// after gameready wrote it.
    Refused { reason: String },

    /// The undo itself failed.
    Failed { error: String },
}

impl UndoOutcome {
    /// Whether this outcome should make the rollback exit non-zero.
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// The words shown to the user.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Reverted { detail } => detail.clone(),
            Self::AlreadyGone => "already gone".to_owned(),
            Self::Left { reason } => reason.clone(),
            Self::Refused { reason } => reason.clone(),
            Self::Failed { error } => error.clone(),
        }
    }
}

/// One operation and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoReport {
    /// The step whose change this reversed.
    pub step: ImprovementId,

    /// What was attempted.
    pub undo: Undo,

    /// How it went.
    pub outcome: UndoOutcome,
}

/// Everything one rollback did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackReport {
    /// The run that was undone.
    pub run: RunId,

    /// One entry per operation, in the order attempted.
    pub undos: Vec<UndoReport>,
}

impl RollbackReport {
    /// How many operations actually reversed something.
    #[must_use]
    pub fn reverted(&self) -> usize {
        self.undos
            .iter()
            .filter(|report| matches!(report.outcome, UndoOutcome::Reverted { .. }))
            .count()
    }

    /// How many failed.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.undos
            .iter()
            .filter(|report| report.outcome.is_failure())
            .count()
    }
}

#[cfg(test)]
#[path = "domain_test.rs"]
mod domain_test;
