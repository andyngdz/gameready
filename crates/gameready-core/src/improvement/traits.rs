//! The contract every improvement implements.

use crate::improvement::domain::{
    ApplyCx, CoreCx, Dependency, ImprovementId, Privilege, Probe, StepPlan, Tag, Verification,
};
use crate::improvement::errors::StepError;
use crate::journal::Change;

/// Identity and metadata, with no execution and no context.
///
/// Split from the lifecycle so the planner, the progress display, and the
/// journal can all handle a step uniformly without caring whether it tunes the
/// system or configures one game.
pub trait Improvement: Send + Sync {
    /// Stable key, never changed once shipped: stored runs reference steps by
    /// this string, so renaming one orphans every undo record naming it.
    fn id(&self) -> ImprovementId;

    /// Short title for the plan and summary screens.
    fn name(&self) -> &str;

    /// Why this is worth doing, shown by `explain` and by `--verbose`.
    /// Written for someone deciding whether to let it run.
    fn rationale(&self) -> &str;

    /// Whether applying needs root. Drives sudo priming and the warning shown
    /// before the first password prompt, so getting this wrong makes the run
    /// prompt at a moment the user was not told about.
    fn privilege(&self) -> Privilege;

    /// Steps that must succeed before this one runs.
    fn requires(&self) -> &[ImprovementId] {
        &[]
    }

    /// Subject areas, for `--only` and for grouping output.
    fn tags(&self) -> &[Tag] {
        &[]
    }

    /// What must already be present for this step to work.
    ///
    /// The executor collects these across every selected step, subtracts what
    /// the system has, shows the remainder in one screen, and installs them
    /// before any step applies. A step therefore never discovers a missing
    /// toolchain halfway through and dies with the system half-changed.
    fn dependencies(&self) -> &[Dependency] {
        &[]
    }
}

/// A system-wide improvement.
///
/// The five methods are one lifecycle: probe to decide, plan to describe, apply
/// to change, verify to prove, rollback to undo. The executor calls them in
/// that order and treats a failed `verify` as a failed `apply`.
pub trait CoreImprovement: Improvement {
    /// Inspects the current state. Must not change anything.
    fn probe(&self, cx: &CoreCx<'_>) -> Result<Probe, StepError>;

    /// Describes what applying would do, for the confirmation screen. Must not
    /// change anything, and must not promise less than `apply` will do.
    fn plan(&self, cx: &CoreCx<'_>) -> Result<StepPlan, StepError>;

    /// Makes the change, recording every mutation through `ApplyCx::mutate`.
    fn apply(&self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError>;

    /// Reads the system back to prove the change took effect.
    ///
    /// Must return at least one check. A step that cannot prove its own effect
    /// is not allowed to report success, and the registry test enforces it.
    ///
    /// What this proves is narrow and worth stating: that the system now reads
    /// back what was written. It does not prove a frame rate improved.
    fn verify(&self, cx: &CoreCx<'_>) -> Result<Verification, StepError>;

    /// Undoes the given changes, which are the ones this step actually
    /// recorded rather than the ones it intended to make.
    fn rollback(&self, undo: &[Change], cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError>;
}
