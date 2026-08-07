//! Which machine a run works against.

use anyhow::{Context as _, Result};
use gameready_core::exec::CommandRunner;
use gameready_core::infra::exec::{FixtureRunner, RealRunner};

use crate::cli::args::Effect;

/// Names a directory shaped like `/` to run against instead of this machine.
///
/// Not documented in `--help`, because it answers a question nobody running
/// gameready on their own computer has. It exists so the CLI's own output can
/// be tested against a fixed machine, and it doubles as a way to reproduce
/// someone else's report without their hardware.
const FAKE_ROOT: &str = "GAMEREADY_FAKE_ROOT";

/// The system a command reads and, when it is the real one, changes.
#[derive(Debug)]
pub enum Machine {
    /// The computer gameready is running on.
    Real(RealRunner),

    /// A checked-in directory standing in for one. Reads answer from it and
    /// every write fails, so nothing a fixture run does can reach a disk.
    Fixture(FixtureRunner),
}

impl Machine {
    /// The machine this run works against, priming the credential cache when
    /// the command will change something.
    ///
    /// A command that only reads falls back to an unprivileged runner, so
    /// `doctor` works in a container with no `sudo` at all.
    pub fn detect(effect: Effect) -> Result<Self> {
        if let Some(root) = std::env::var_os(FAKE_ROOT) {
            let fixture = FixtureRunner::open(std::path::PathBuf::from(root))
                .with_context(|| format!("{FAKE_ROOT} does not name a usable fixture"))?;
            return Ok(Self::Fixture(fixture));
        }

        match effect {
            Effect::Reads => Ok(Self::Real(
                RealRunner::detect().unwrap_or_else(|_| RealRunner::unprivileged()),
            )),
            Effect::Mutates => Ok(Self::Real(
                RealRunner::detect().context("no way to run privileged commands was found")?,
            )),
        }
    }

    /// What steps read the system through.
    #[must_use]
    pub fn runner(&self) -> &dyn CommandRunner {
        match self {
            Self::Real(runner) => runner,
            Self::Fixture(runner) => runner,
        }
    }

    /// Fills the credential cache, prompting once.
    ///
    /// Called after every question a command has, so the password prompt is the
    /// last thing between deciding and doing rather than the first thing a user
    /// meets. A fixture has nothing to authorise: it cannot be written to.
    pub fn authorize(&self) -> Result<()> {
        match self {
            Self::Real(runner) => runner
                .prime()
                .context("could not get permission to make system changes"),
            Self::Fixture(_) => Ok(()),
        }
    }
}

#[cfg(test)]
#[path = "runtime_test.rs"]
mod runtime_test;
