//! A whole machine, checked in as a directory.
//!
//! Everything a step reads goes through [`CommandRunner`], so a directory
//! shaped like `/` plus a table of command answers is indistinguishable from a
//! real system as far as any step can tell. That is what lets the output of
//! `doctor`, `explain`, and a dry run be snapshotted: the machine under them
//! stops changing between runs and between machines.
//!
//! Read-only on purpose. A snapshot test that mutates has escaped the fixture,
//! and every write here fails loudly rather than quietly touching the real
//! filesystem.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::exec::{CmdOutput, ExecError};

/// The file that says how commands answer, at the root of a fixture.
const MANIFEST: &str = "commands.toml";

/// A fake machine rooted at a directory.
#[derive(Debug)]
pub struct FixtureRunner {
    root: PathBuf,
    binaries: Vec<String>,
    answers: HashMap<String, CmdOutput>,
}

impl FixtureRunner {
    /// Opens the fixture at `root`, reading its command table.
    ///
    /// A fixture with no `commands.toml` is still usable: it answers every
    /// command with success and empty output, which is enough for a machine
    /// whose steps only read files.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ExecError> {
        let root = root.into();
        let manifest = Manifest::read(&root.join(MANIFEST))?;

        Ok(Self {
            root,
            binaries: manifest.binaries,
            answers: manifest
                .commands
                .into_iter()
                .map(|answer| (answer.line.clone(), answer.output()))
                .collect(),
        })
    }

    /// Where a path on the fake machine actually lives.
    ///
    /// An absolute path is re-rooted into the fixture. A relative one is
    /// joined as it stands, which is what a step asking for something under the
    /// working directory means.
    pub(super) fn resolve(&self, path: &Path) -> PathBuf {
        path.strip_prefix("/")
            .map_or_else(|_| self.root.join(path), |rest| self.root.join(rest))
    }

    /// What the fixture says a command does.
    ///
    /// An unlisted command succeeds with no output, so a fixture only has to
    /// carry the commands whose answers a test depends on.
    pub(super) fn answer(&self, rendered: &str) -> CmdOutput {
        self.answers.get(rendered).cloned().unwrap_or(CmdOutput {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        })
    }

    /// Whether the fixture claims this binary is on `PATH`.
    pub(super) fn has_binary(&self, name: &str) -> bool {
        self.binaries.iter().any(|binary| binary == name)
    }
}

/// The parsed `commands.toml`.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    /// Executables the fake machine has on `PATH`.
    #[serde(default)]
    binaries: Vec<String>,

    /// Commands whose answer a test depends on.
    #[serde(default)]
    commands: Vec<Answer>,
}

impl Manifest {
    /// Reads the table, treating an absent file as an empty one.
    fn read(path: &Path) -> Result<Self, ExecError> {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Ok(Self::default());
        };

        toml::from_str(&text).map_err(|source| ExecError::Read {
            path: path.to_path_buf(),
            source: std::io::Error::other(source.to_string()),
        })
    }
}

/// One command and what running it does.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    /// The command as a user would type it, matching what `Cmd` displays.
    line: String,

    #[serde(default)]
    stdout: String,

    #[serde(default)]
    stderr: String,

    /// The exit status. Absent means it worked.
    #[serde(default)]
    code: i32,
}

impl Answer {
    fn output(&self) -> CmdOutput {
        CmdOutput {
            code: self.code,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
        }
    }
}

#[cfg(test)]
#[path = "fixture_runner_test.rs"]
mod fixture_runner_test;
