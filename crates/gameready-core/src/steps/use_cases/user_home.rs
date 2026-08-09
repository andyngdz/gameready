//! Where the invoking user's files live.

use std::path::PathBuf;

/// The variable every login shell sets to the user's own directory.
const HOME_VAR: &str = "HOME";

/// Where to look when `$HOME` is unset.
///
/// Unset `$HOME` in practice means a service manager or a root shell that never
/// sourced a profile. Falling back to root's home keeps the steps that write
/// per-user files pointing at a real directory rather than a relative path that
/// would land wherever the process happened to start.
const HOMELESS_FALLBACK: &str = "/root";

/// The directory a step writing a per-user file should write under.
///
/// One reader so the three steps that keep files in the user's home agree on
/// where that is, and so the fallback is decided once rather than per step.
#[must_use]
pub fn user_home() -> PathBuf {
    PathBuf::from(std::env::var(HOME_VAR).unwrap_or_else(|_| HOMELESS_FALLBACK.to_owned()))
}

#[cfg(test)]
#[path = "user_home_test.rs"]
mod user_home_test;
