//! Command names shared by the exec and package adapters.

/// Spelled the same in two unrelated places, so it is named once here rather
/// than twice with different names: coreutils' `install` moves a staged file
/// into a root-owned path, and `install` is also the apt and dnf subcommand.
/// A shared literal, not a shared concept.
pub const INSTALL: &str = "install";

/// The directory a fake machine claims its executables live in.
///
/// Both the mock and the fixture answer `which` with a path rather than a bare
/// name, because a caller may go on to use it. Neither has a real one, so they
/// give the same plausible answer.
pub const FAKE_BIN_DIR: &str = "/usr/bin";
