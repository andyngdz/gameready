//! Text shared across the subcommands.

/// Context added when probing the machine fails.
///
/// Named once because three subcommands attach it, and a reworded copy in one
/// of them would make the same failure read differently depending on which
/// command hit it.
pub const CANNOT_READ_SYSTEM: &str = "could not read this system";

/// A stand-in `/etc/os-release`, so command tests can build a system the
/// probe accepts without repeating the file in every module.
#[cfg(test)]
pub(super) const OS_RELEASE_FIXTURE: &str = indoc::indoc! {r#"
    ID=ubuntu
    ID_LIKE=debian
    VERSION_ID="26.04"
    PRETTY_NAME="Ubuntu 26.04 LTS"
    "#};

/// Reported when the journal cannot be opened. Named once because both `apply`
/// and `init` open it, and a run that cannot journal must not proceed.
pub const CANNOT_OPEN_JOURNAL: &str = "could not open the journal";
