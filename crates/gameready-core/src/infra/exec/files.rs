//! Writing files on the real filesystem.

use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use crate::exec::ExecError;

/// Writes a file only its owner can read, creating any missing directories.
///
/// The mode is set as the file is created, not afterwards. Creating it readable
/// and tightening it later leaves a window in which whatever is inside can be
/// read by anyone on the machine, which for a copy of Steam's config means an
/// encrypted app ticket and a cloud key.
pub fn write_owner_only(path: &Path, contents: &str) -> Result<(), ExecError> {
    ensure_parent(path)?;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|source| ExecError::Write {
            path: path.to_path_buf(),
            source,
        })?;

    file.write_all(contents.as_bytes())
        .map_err(|source| ExecError::Write {
            path: path.to_path_buf(),
            source,
        })
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

/// Creates the directories a path needs before it can be written.
fn ensure_parent(path: &Path) -> Result<(), ExecError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|source| ExecError::Write {
        path: parent.to_path_buf(),
        source,
    })
}

#[cfg(test)]
#[path = "files_test.rs"]
mod files_test;
