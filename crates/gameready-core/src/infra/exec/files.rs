//! Writing files on the real filesystem.

use std::path::{Path, PathBuf};

use crate::exec::{Cmd, ExecError};
use crate::infra::exec::constants::INSTALL;

/// Moves a staged file into place as root, creating the directories above it.
///
/// `-D` is load bearing. The drop-in directories a step writes into do not
/// exist until something makes them, and plain `install` answers "No such file
/// or directory" rather than creating one. The unprivileged path through
/// `write_file` already calls `create_dir_all`, so without this the two differ
/// on whether a step may write a new directory.
pub fn install_command(staged: &Path, destination: &Path) -> Cmd {
    Cmd::root(INSTALL)
        .arg("-D")
        .arg("-m")
        .arg("0644")
        .arg(staged.to_string_lossy().into_owned())
        .arg(destination.to_string_lossy().into_owned())
}

/// Stages content in a temporary file, for a privileged `install` to move into
/// place.
pub fn stage_temp_file(destination: &Path, contents: &str) -> Result<PathBuf, ExecError> {
    let name = destination.file_name().map_or_else(
        || "gameready".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let staged = std::env::temp_dir().join(format!("gameready-staged-{name}"));
    std::fs::write(&staged, contents).map_err(|source| ExecError::Write {
        path: staged.clone(),
        source,
    })?;
    Ok(staged)
}

#[cfg(test)]
#[path = "files_test.rs"]
mod files_test;
