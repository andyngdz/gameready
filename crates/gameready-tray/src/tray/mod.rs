//! The panel indicator: what it shows, and how it reads the machine.

mod domain;
mod service;

pub use domain::{Activity, Row, RowAction, Snapshot};
pub(crate) use domain::{PROTON_GE_ID, PROTON_GE_STEP_ID};
pub use service::{sweep, sweep_game};
