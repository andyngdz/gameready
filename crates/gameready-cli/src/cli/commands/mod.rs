//! One module per subcommand. Each maps arguments to a core call and returns
//! text; `main` owns the write to stdout and the exit code.

mod apply;
mod constants;
mod doctor;
mod explain;
pub(crate) mod init;
mod list_games;
mod rollback;
mod selection;
mod selftest;

/// Shared by the command tests that assert the password prompt came before the
/// first privileged command.
#[cfg(test)]
#[path = "prompt_recorder_test.rs"]
pub(crate) mod prompt_recorder;

pub use apply::run as apply;
pub use doctor::run as doctor;
pub use explain::run as explain;
pub use init::{run as init, InitRequest};
pub use list_games::run as list_games;
pub use rollback::run as rollback;
pub use selftest::run as selftest;
