//! Text shared across the subcommands.

/// Context added when probing the machine fails.
///
/// Named once because three subcommands attach it, and a reworded copy in one
/// of them would make the same failure read differently depending on which
/// command hit it.
pub const CANNOT_READ_SYSTEM: &str = "could not read this system";
