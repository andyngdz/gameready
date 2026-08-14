//! Domain types for the run feature.

mod contested;
mod plan;
mod preflight;
mod report;
mod selftest;

pub use contested::Contested;
pub use plan::{Deferred, PlannedInstall, RunPlan};
pub use preflight::{DependencyStatus, InstallConsent, PreflightReport, ResolvedDependency};
pub use report::{MissingDependency, Mode, RunEvent, RunReport, RunStatus, StepReport};
pub use selftest::{Phase, RevertCheck, SelftestResult, StepSelftest};
