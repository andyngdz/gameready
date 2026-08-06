//! Writing values into sysfs attributes.
//!
//! A sysfs node is written in place, not created, so the `install`-based file
//! write the rest of the runner uses does not apply. A root-owned attribute is
//! reached by streaming the value to a privileged writer over stdin, which
//! keeps the value off the command line and needs no shell.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::exec::{Escalator, ExecError};
use crate::improvement::Privilege;

/// The program that copies its stdin into the file named as its argument.
const STDIN_WRITER: &str = "tee";

/// Writes `value` into the sysfs attribute at `path`.
///
/// A user-owned attribute is written directly; a root-owned one goes through
/// the escalator. The value is never placed on a command line.
pub fn write_sysfs_value(
    escalator: &Escalator,
    path: &Path,
    value: &str,
    privilege: Privilege,
) -> Result<(), ExecError> {
    match privilege {
        Privilege::User => std::fs::write(path, value).map_err(|source| ExecError::Write {
            path: path.to_path_buf(),
            source,
        }),
        Privilege::Root => write_as_root(escalator, path, value),
    }
}

/// Streams `value` to a privileged writer that lands it in `path`.
///
/// The writer echoes its stdin to stdout, which is discarded. A non-zero exit
/// becomes an error carrying the writer's stderr.
fn write_as_root(escalator: &Escalator, path: &Path, value: &str) -> Result<(), ExecError> {
    let (program, args) = escalator.wrap(STDIN_WRITER, &[path.to_string_lossy().into_owned()]);

    let mut child = Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| ExecError::Spawn {
            command: path.display().to_string(),
            source,
        })?;

    child
        .stdin
        .take()
        .ok_or_else(|| ExecError::Write {
            path: path.to_path_buf(),
            source: std::io::Error::other("privileged writer stdin was not captured"),
        })?
        .write_all(value.as_bytes())
        .map_err(|source| ExecError::Write {
            path: path.to_path_buf(),
            source,
        })?;

    let output = child
        .wait_with_output()
        .map_err(|source| ExecError::Spawn {
            command: path.display().to_string(),
            source,
        })?;

    if output.status.success() {
        return Ok(());
    }
    Err(ExecError::NonZeroExit {
        command: path.display().to_string(),
        code: output.status.code().unwrap_or(1),
        stdout: String::new(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

#[cfg(test)]
#[path = "sysfs_test.rs"]
mod sysfs_test;
