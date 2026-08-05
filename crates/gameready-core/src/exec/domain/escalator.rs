//! Becoming root for one command at a time.
//!
//! gameready never runs as root. The process stays as the invoking user and
//! prefixes an escalator onto the individual commands that need privilege, so
//! terminal rendering, config parsing, and Steam file handling never execute
//! with more authority than they need.

use std::fmt;

use itertools::Itertools;

use crate::exec::constants::{DOAS, PKEXEC, RUN0, SUDO};
use crate::exec::domain::command::Cmd;
use crate::exec::errors::ExecError;

/// The tools that can raise one command to root, in preference order.
///
/// `sudo` first because it is what almost every target distro ships and the
/// only one with a credential cache users already understand. `pkexec` last
/// because it prompts through a desktop agent rather than the terminal, which
/// is jarring in the middle of a terminal run.
const CANDIDATES: [Escalator; 4] = [
    Escalator::Sudo,
    Escalator::Doas,
    Escalator::Run0,
    Escalator::Pkexec,
];

/// A way to run one command as root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escalator {
    /// The common case. Caches credentials, so priming once means the rest of
    /// the run does not prompt.
    Sudo,
    /// OpenBSD's simpler alternative.
    Doas,
    /// systemd's escalation.
    Run0,
    /// polkit's, used when nothing else is available.
    Pkexec,
}

impl Escalator {
    /// The binary name to look for and prefix.
    #[must_use]
    pub const fn binary(self) -> &'static str {
        match self {
            Self::Sudo => SUDO,
            Self::Doas => DOAS,
            Self::Run0 => RUN0,
            Self::Pkexec => PKEXEC,
        }
    }

    /// Whether this tool caches credentials between commands.
    ///
    /// Only `sudo` does, which is why the "asked once" promise is only true
    /// there. On the others the user is prompted per command, and the
    /// pre-flight screen says so rather than promising otherwise.
    #[must_use]
    pub const fn caches_credentials(self) -> bool {
        matches!(self, Self::Sudo)
    }

    /// The command that primes the credential cache without doing anything.
    ///
    /// `None` for tools with no cache to prime.
    #[must_use]
    pub fn prime(self) -> Option<Cmd> {
        match self {
            Self::Sudo => Some(Cmd::user(SUDO).arg("-v")),
            Self::Doas | Self::Run0 | Self::Pkexec => None,
        }
    }

    /// The command that answers "is the cache warm right now" without
    /// prompting.
    ///
    /// Used to detect a `timestamp_timeout=0` configuration, where the "asked
    /// once" promise cannot be kept and so must not be made.
    #[must_use]
    pub fn probe_cached(self) -> Option<Cmd> {
        match self {
            Self::Sudo => Some(Cmd::user(SUDO).arg("-n").arg("true")),
            Self::Doas | Self::Run0 | Self::Pkexec => None,
        }
    }

    /// Wraps a command so it runs as root.
    ///
    /// `sudo -n` is deliberate: after priming, a command that would prompt
    /// fails instead. That surfaces a stale cache as an error rather than as a
    /// password prompt appearing underneath a progress display, where the user
    /// cannot see what is being asked.
    #[must_use]
    pub fn wrap(self, program: &str, args: &[String]) -> (String, Vec<String>) {
        let mut wrapped = Vec::with_capacity(args.len() + 2);
        if self == Self::Sudo {
            wrapped.push("-n".to_owned());
        }
        wrapped.push(program.to_owned());
        wrapped.extend_from_slice(args);
        (self.binary().to_owned(), wrapped)
    }

    /// Picks the first available escalator.
    ///
    /// `lookup` answers whether a binary is on `PATH`; taking it as an argument
    /// keeps this testable without a real filesystem.
    pub fn detect(lookup: impl Fn(&str) -> bool) -> Result<Self, ExecError> {
        CANDIDATES
            .into_iter()
            .find(|candidate| lookup(candidate.binary()))
            .ok_or_else(|| ExecError::NoEscalator {
                looked_for: CANDIDATES
                    .iter()
                    .map(|candidate| candidate.binary())
                    .join(", "),
            })
    }

    /// A fallback for commands that only read. Root commands will fail at
    /// runtime with a spawn error since no escalator binary exists.
    #[must_use]
    pub const fn fallback_unprivileged() -> Self {
        Self::Sudo
    }
}

impl fmt::Display for Escalator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.binary())
    }
}

#[cfg(test)]
#[path = "escalator_test.rs"]
mod escalator_test;
