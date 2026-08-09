//! The panel indicator: what it shows, and how it reads the machine.

mod domain;
mod service;

pub use domain::{Activity, Row, Snapshot};
pub use service::{sweep, sweep_game};
