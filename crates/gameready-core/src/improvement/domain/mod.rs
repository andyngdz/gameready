//! Types every improvement is described and reported in.

mod context;
mod dependency;
mod identity;
mod outcome;
mod plan;
mod verify;

pub use context::{ApplyCx, CoreCx};
pub use dependency::{Dependency, DependencyKind, KernelVersion, PackageSpec};
pub use identity::{ImprovementId, Privilege, Tag};
pub use outcome::{Outcome, OutcomeKind, Probe, RollbackStatus, SkipReason};
pub use plan::{PlannedAction, StepPlan};
pub use verify::{Check, Verification};
