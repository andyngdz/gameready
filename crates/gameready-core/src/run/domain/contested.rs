//! A step the probe found in conflict, where the run can take the seat back
//! cleanly if the user says so.

use crate::improvement::CoreImprovement;

/// The one trait-object field both hand-written `Debug` impls in this module
/// name, shared so the two cannot drift.
pub(super) const STEP_FIELD: &str = "step";

/// Neither settled nor pending: settling it would skip a decision the user is
/// entitled to make, and applying it would take over something the user never
/// agreed to. The run asks, and the answer moves it one way or the other.
pub struct Contested {
    /// The step itself, still live so it can apply once the user agrees.
    pub step: Box<dyn CoreImprovement>,

    /// What owns the setting, for the question to name it.
    pub with: String,

    /// The full conflict detail, reported when the user declines.
    pub detail: String,
}

impl std::fmt::Debug for Contested {
    /// Hand-written because `CoreImprovement` is a trait object with no
    /// `Debug` bound, and adding one would force it on every step for the sake
    /// of a test assertion.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Contested")
            .field(STEP_FIELD, &self.step.id())
            .field("with", &self.with)
            .finish()
    }
}
