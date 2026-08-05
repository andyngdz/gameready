//! Running a set of improvements and reporting what happened.

mod domain;
mod errors;
mod service;
pub(crate) mod use_cases;

pub use domain::{
    DependencyStatus, MissingDependency, Mode, PreflightReport, ResolvedDependency, RunEvent,
    RunReport, RunStatus, StepReport,
};
pub use errors::RunError;
pub use service::execute;
pub use use_cases::resolve::resolve_dependencies;
