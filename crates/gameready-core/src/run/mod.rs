//! Running a set of improvements and reporting what happened.

mod domain;
mod errors;
mod service;
pub(crate) mod use_cases;

pub use domain::{
    DependencyStatus, InstallConsent, MissingDependency, Mode, Phase, PlannedInstall,
    PreflightReport, ResolvedDependency, RevertCheck, RunEvent, RunPlan, RunReport, RunStatus,
    SelftestResult, StepReport, StepSelftest,
};
pub use errors::RunError;
pub use service::{apply_plan, execute};
pub use use_cases::compat::compat_targets_for;
pub use use_cases::launch::targets_for;
pub use use_cases::plan::plan_run;
pub use use_cases::resolve::resolve_dependencies;
pub use use_cases::selftest::selftest;
