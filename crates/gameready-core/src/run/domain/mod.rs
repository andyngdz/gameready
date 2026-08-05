//! Domain types for the run feature.

mod preflight;
mod report;

pub use preflight::{DependencyStatus, PreflightReport, ResolvedDependency};
pub use report::{MissingDependency, Mode, RunEvent, RunReport, RunStatus, StepReport};
