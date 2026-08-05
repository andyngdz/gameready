//! The seam between step logic and the real system.

mod constants;
mod domain;
mod errors;
mod traits;

pub use domain::{Cmd, CmdOutput, Escalator};
pub use errors::ExecError;
pub use traits::CommandRunner;
