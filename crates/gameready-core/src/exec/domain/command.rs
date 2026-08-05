//! A command to run, and what came back.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::exec::constants::SUDO;
use crate::improvement::Privilege;

/// One process invocation.
///
/// Held as program plus argument vector, never as a shell string. There is no
/// shell involved anywhere in this crate: a package name interpolated into a
/// shell string is an injection waiting for the first game whose profile name
/// contains a quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cmd {
    program: String,
    args: Vec<String>,
    privilege: Privilege,
}

impl Cmd {
    /// A command run as the invoking user.
    #[must_use]
    pub fn user(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            privilege: Privilege::User,
        }
    }

    /// A command that needs root. The runner prefixes the configured escalator
    /// rather than the process ever running as root itself.
    #[must_use]
    pub fn root(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            privilege: Privilege::Root,
        }
    }

    /// Appends one argument.
    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Appends several arguments.
    #[must_use]
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// The executable name or path.
    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The arguments, without the program name.
    #[must_use]
    pub fn arguments(&self) -> &[String] {
        &self.args
    }

    /// Whether this needs root.
    #[must_use]
    pub const fn privilege(&self) -> Privilege {
        self.privilege
    }

    /// Whether this needs root.
    #[must_use]
    pub const fn needs_root(&self) -> bool {
        matches!(self.privilege, Privilege::Root)
    }
}

impl fmt::Display for Cmd {
    /// Renders the command as a user would type it, for logs and for the
    /// "here is exactly what will run" screen. Arguments containing spaces are
    /// quoted so the line can be pasted into a shell and behave the same way.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.needs_root() {
            f.write_str(SUDO)?;
            f.write_str(" ")?;
        }
        f.write_str(&self.program)?;
        for arg in &self.args {
            if arg.contains(char::is_whitespace) {
                write!(f, " '{arg}'")?;
            } else {
                write!(f, " {arg}")?;
            }
        }
        Ok(())
    }
}

/// What a finished command produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmdOutput {
    /// Exit status. Zero means success; the runner turns non-zero into an
    /// error, so a caller holding this has already succeeded.
    pub code: i32,

    /// Everything the command wrote to stdout, with the trailing newline kept.
    pub stdout: String,

    /// Everything the command wrote to stderr.
    pub stderr: String,
}

impl CmdOutput {
    /// Stdout with surrounding whitespace removed, which is what reading a
    /// single value out of a command almost always wants.
    #[must_use]
    pub fn stdout_trimmed(&self) -> &str {
        self.stdout.trim()
    }
}
