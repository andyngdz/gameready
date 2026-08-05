//! Domain types for the run feature.

mod preflight;
mod report;
mod selftest;

pub use preflight::{DependencyStatus, PreflightReport, ResolvedDependency};
pub use report::{MissingDependency, Mode, RunEvent, RunReport, RunStatus, StepReport};
pub use selftest::{Phase, RevertCheck, SelftestResult, StepSelftest};
