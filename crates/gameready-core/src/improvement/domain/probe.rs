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

    /// Installed, but a newer release exists. Usable now, and worth updating.
    ///
    /// `installed` is the tag the machine already has and `latest` the newest
    /// one, so a row can say both in one sentence.
    UpdateAvailable { installed: String, latest: String },

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

/// What a probe result means to whatever draws it.
///
/// The words of a probe are one thing and its colour is another. A terminal
/// gutter and a panel menu agree on which of four buckets a result falls in,
/// and disagree on everything after that. The bucket lives here so the glyph,
/// the colour, and the pixmap can each stay with the surface that owns them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    /// Found already in the desired state.
    Set,

    /// Installed and usable, but a newer release exists.
    UpdateAvailable,

    /// Not applied, and this system can take it.
    Ready,

    /// Something else owns the setting. Not a fault: the owner is the user's
    /// own choice, and a surface can offer to take it over without implying
    /// anything is broken.
    Conflict,

    /// Probing itself failed, or the state could not be read. The one status a
    /// user has to act on, because nothing on the machine explains it.
    Attention,

    /// Ruled out, or unreadable. Either way nothing will happen here.
    Inactive,
}

impl Probe {
    /// Which of the four display buckets this result falls in.
    #[must_use]
    pub const fn status(&self) -> ProbeStatus {
        match self {
            Self::AlreadyApplied { .. } => ProbeStatus::Set,
            Self::UpdateAvailable { .. } => ProbeStatus::UpdateAvailable,
            Self::Applicable => ProbeStatus::Ready,
            Self::Conflict { .. } => ProbeStatus::Conflict,
            Self::NotApplicable { .. } | Self::Unknown { .. } => ProbeStatus::Inactive,
        }
    }

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
            Self::UpdateAvailable { installed, latest } => {
                format!("installed {installed}, {latest} available")
            }
            Self::NotApplicable { reason } => format!("not applicable, {reason}"),
            Self::Conflict { detail, .. } => detail.clone(),
            Self::Unknown { reason } => format!("could not tell, {reason}"),
        }
    }
}

#[cfg(test)]
#[path = "probe_test.rs"]
mod probe_test;
