//! Running a set of improvements and reporting what happened.

mod domain;
mod errors;
mod service;

pub use domain::{MissingDependency, Mode, RunEvent, RunReport, RunStatus, StepReport};
pub use errors::RunError;
pub use service::execute;
