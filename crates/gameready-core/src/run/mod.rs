//! Running a set of improvements and reporting what happened.

mod domain;
mod errors;
mod service;
pub(crate) mod use_cases;

pub use domain::{
    DependencyStatus, MissingDependency, Mode, Phase, PreflightReport, ResolvedDependency,
    RevertCheck, RunEvent, RunReport, RunStatus, SelftestResult, StepReport, StepSelftest,
};
pub use errors::RunError;
pub use service::execute;
pub use use_cases::launch::targets_for;
pub use use_cases::resolve::resolve_dependencies;
pub use use_cases::selftest::selftest;
