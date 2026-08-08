//! The seam between step logic and the real system.
//!
//! Every process spawn and every filesystem touch a step performs goes through
//! this trait. That is what makes a step testable without root and without a
//! real machine: the mock implementation records what was asked for and answers
//! from a fixture, so a step's command sequence, journal records, and rollback
//! path are all covered by ordinary unit tests.

use std::path::{Path, PathBuf};

use crate::exec::domain::{Cmd, CmdOutput};
use crate::exec::errors::ExecError;
use crate::improvement::Privilege;

/// Runs commands and touches files on behalf of a step.
///
/// Implementations: `RealRunner` against the live system, `DryRunner` which
/// answers reads and refuses writes, and `MockRunner` for tests.
///
/// Steps must not mutate through this directly. Mutations go through
/// `ApplyCx::mutate`, which makes the undo record durable first; a clippy
/// `disallowed_methods` entry catches the direct route.
pub trait CommandRunner: Send + Sync {
    /// Runs a command to completion and returns its output.
    ///
    /// A non-zero exit is an error, not an output with a code set: a caller
    /// holding a [`CmdOutput`] can assume the command worked. Commands whose
    /// non-zero exit is meaningful, such as a package-manager query answering
    /// "not installed", use [`CommandRunner::run_allowing_failure`].
    fn run(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError>;

    /// Runs a command and returns its output whatever the exit status.
    ///
    /// For probes where a non-zero exit is an answer rather than a fault, such
    /// as `dpkg-query -s <pkg>` on a package that is not installed.
    fn run_allowing_failure(&self, cmd: &Cmd) -> Result<CmdOutput, ExecError>;

    /// Reads a file as UTF-8.
    fn read_to_string(&self, path: &Path) -> Result<String, ExecError>;

    /// Lists a directory's immediate entries, as full paths, in sorted order.
    ///
    /// Sorted so a step that walks it reads the same list on every run. Used to
    /// enumerate `/sys/block`, whose device entries are not known ahead of time
    /// and cannot be hardcoded.
    fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>, ExecError>;

    /// Writes a file, creating it if absent.
    ///
    /// For a root-owned destination the implementation routes through the
    /// escalator rather than requiring the process to be root.
    fn write_file(
        &self,
        path: &Path,
        contents: &str,
        privilege: Privilege,
    ) -> Result<(), ExecError>;

    /// Writes a value into an existing sysfs attribute.
    ///
    /// Separate from [`CommandRunner::write_file`] because a sysfs node is
    /// written, not created: the `install` a root file write lands through
    /// cannot touch one. The write goes through the escalator for a root-owned
    /// attribute, the same as any other privileged action.
    fn write_sysfs(&self, path: &Path, value: &str, privilege: Privilege) -> Result<(), ExecError>;

    /// Writes a file only its owner can read.
    ///
    /// For a pre-image of something that holds credentials. Steam's
    /// `localconfig.vdf` carries an encrypted app ticket and a cloud key, and
    /// Steam leaves its own copy group-readable; a backup gameready keeps for
    /// every run, in a directory nothing prunes, should not repeat that.
    ///
    /// Always the invoking user. A root-owned backup could not be read back by
    /// the rollback that needs it.
    fn write_private_file(&self, path: &Path, contents: &str) -> Result<(), ExecError>;

    /// Deletes a file. Succeeds if it is already gone, so rollback is
    /// idempotent and a half-finished undo can be re-run.
    fn remove_file(&self, path: &Path, privilege: Privilege) -> Result<(), ExecError>;

    /// Whether a path exists. Answers `false` for a path that exists but cannot
    /// be stat'd, which is the useful answer for probing.
    fn path_exists(&self, path: &Path) -> bool;

    /// Fetches a URL into a file, reporting how much has landed as it goes.
    ///
    /// Its own method rather than one more command for two reasons. A download
    /// is the only thing a run does whose end is knowable while it is still
    /// happening, and a process that is started and waited on cannot say how
    /// far it got. And a fixture run has to be able to refuse it: reads answer
    /// from a directory, and a fetch that reached the network anyway would make
    /// the fixture a fiction.
    ///
    /// Always the invoking user. Nothing gameready downloads lands outside the
    /// user's own directories.
    fn download(&self, url: &str, dest: &Path, on_bytes: &dyn Fn(u64)) -> Result<(), ExecError>;

    /// Resolves an executable on `PATH`.
    ///
    /// Used to probe binary dependencies. Deliberately not "ask the package
    /// manager": a user may have installed a tool outside it, and a step needs
    /// to know whether the binary is usable, not whether a package is recorded.
    fn which(&self, binary: &str) -> Option<PathBuf>;
}
