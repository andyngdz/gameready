//! Types every improvement is described and reported in.

mod context;
mod dependency;
mod identity;
mod outcome;
mod plan;
mod probe;
mod trouble;
mod verify;

pub use context::{ApplyCx, CoreCx};
pub use dependency::{Dependency, DependencyKind, KernelVersion, PackageSpec};
pub use identity::{ImprovementId, Privilege, Tag};
pub use outcome::{Outcome, OutcomeKind, RollbackStatus, SkipReason};
pub use plan::{PlannedAction, PlannedPackage, StepPlan};
pub use probe::Probe;
pub use trouble::{Remedy, Trouble};
pub use verify::{Check, Verification};
