//! One module per subcommand. Each maps arguments to a core call and returns
//! text; `main` owns the write to stdout and the exit code.

mod apply;
mod constants;
mod doctor;
mod rollback;
mod selftest;

pub use apply::run as apply;
pub use doctor::run as doctor;
pub use rollback::run as rollback;
pub use selftest::run as selftest;
