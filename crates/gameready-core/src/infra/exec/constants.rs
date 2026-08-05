//! Command names shared by the exec and package adapters.

/// Spelled the same in two unrelated places, so it is named once here rather
/// than twice with different names: coreutils' `install` moves a staged file
/// into a root-owned path, and `install` is also the apt and dnf subcommand.
/// A shared literal, not a shared concept.
pub const INSTALL: &str = "install";
