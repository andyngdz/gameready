//! Implementations of [`crate::exec::CommandRunner`].

mod dry_runner;
mod real_runner;

#[cfg(any(test, feature = "testkit"))]
mod mock_runner;

pub use dry_runner::DryRunner;
pub use real_runner::RealRunner;

#[cfg(any(test, feature = "testkit"))]
pub use mock_runner::MockRunner;
