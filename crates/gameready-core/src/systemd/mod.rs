//! Reading what systemd units are doing on this machine.
//!
//! Read-only for now. Steps that enable or disable a unit land with the
//! scheduler work; until one exists, a writer here would be code with no
//! caller and no test that runs it.

mod constants;
mod domain;
mod errors;
mod service;

pub use domain::UnitState;
pub use errors::SystemdError;
pub use service::unit_state;
