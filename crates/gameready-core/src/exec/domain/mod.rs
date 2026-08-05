//! What a command is, and how one becomes privileged.

pub(crate) mod command;
mod escalator;

pub use command::{Cmd, CmdOutput};
pub use escalator::Escalator;
