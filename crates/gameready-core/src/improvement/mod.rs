//! What an improvement is, and the contract every one implements.

mod domain;
mod errors;
mod traits;

pub use domain::{
    ApplyCx, Check, CoreCx, Dependency, DependencyKind, ImprovementId, KernelVersion, Outcome,
    OutcomeKind, PackageSpec, PlannedAction, PlannedPackage, Privilege, Probe, Remedy,
    RollbackStatus, SkipReason, StepPlan, Tag, Trouble, Verification,
};
pub use errors::{ImprovementIdError, ParseFailure, StepError};
pub use traits::{CoreImprovement, Improvement};
