//! Implementations of [`crate::exec::CommandRunner`].

pub(crate) mod constants;
mod dry_runner;
mod real_runner;

#[cfg(any(test, feature = "testkit"))]
pub(crate) mod mock_runner;
#[cfg(any(test, feature = "testkit"))]
mod mock_runner_impl;

pub use dry_runner::DryRunner;
pub use real_runner::RealRunner;

#[cfg(any(test, feature = "testkit"))]
pub use mock_runner::MockRunner;
