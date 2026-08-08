//! What probing a step found, before anything is changed.

use serde::{Deserialize, Serialize};

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
    /// A phrase rather than a sentence, and never parenthesised: every caller
    /// puts it after a name and a separator, where a bracket reads as an aside
    /// and the line's whole point is what is inside it.
    ///
    /// Lives here rather than in the CLI for the same reason as
    /// [`crate::improvement::Outcome::detail`]: what there is to say about a
    /// probe result is a property of the result. The CLI decides the layout,
    /// this decides the words, and `doctor` and the plan screen cannot drift
    /// apart.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Applicable => "would apply".to_owned(),
            Self::AlreadyApplied { evidence } => format!("already set, {evidence}"),
            Self::NotApplicable { reason } => format!("not applicable, {reason}"),
            Self::Conflict { detail, .. } => detail.clone(),
            Self::Unknown { reason } => format!("could not tell, {reason}"),
        }
    }
}

#[cfg(test)]
#[path = "probe_test.rs"]
mod probe_test;
