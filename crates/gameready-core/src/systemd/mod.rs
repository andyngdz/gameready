//! Reading what systemd units are doing on this machine, and naming the verbs
//! that change them.
//!
//! The queries live here. The writes are performed by whichever step owns the
//! unit, because a unit change belongs in that step's journal record; this
//! module owns only the names, so two callers cannot spell `--now` differently.

mod constants;
mod domain;
mod errors;
mod service;

pub use constants::{DISABLE, ENABLE, NOW, RESTART, STOP, SYSTEMCTL};
pub use domain::UnitState;
pub use errors::SystemdError;
pub use service::unit_state;
