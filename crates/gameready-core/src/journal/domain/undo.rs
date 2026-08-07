//! The operations that reverse a [`Change`](super::Change).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::improvement::Privilege;

use super::change::assumed_root;

/// What a systemd unit's state was before the run.
///
/// An enum rather than a bool so the undo reads as "it was disabled, put it
/// back" instead of a bare `if enabled`, where the false branch is the one that
/// acts and is easy to invert by accident.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriorUnitState {
    /// The run enabled it, so the undo disables it again.
    WasDisabled,
    /// It was already enabled, so the undo leaves it alone.
    WasEnabled,
}

/// The operation that reverses a [`Change`](super::Change).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "undo", rename_all = "snake_case")]
pub enum Undo {
    /// Delete a file gameready created. Refuses if the file no longer matches
    /// `expect_sha256`, because the user edited it and clobbering their edit is
    /// worse than leaving the file behind.
    DeleteFile {
        path: PathBuf,
        expect_sha256: String,
        /// The privilege the write was made with, so the undo matches it.
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },

    /// Put a pre-image back, restoring the recorded mode.
    RestoreFile {
        path: PathBuf,
        from: PathBuf,
        mode: u32,
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },

    /// Set a kernel parameter back to its prior value.
    SetSysctl { key: String, value: String },

    /// Write a sysfs attribute back to its prior value.
    WriteSysfs { path: PathBuf, value: String },

    /// Report packages left installed. Performs nothing unless the caller opted
    /// into removal.
    ReportPackages {
        manager: String,
        installed: Vec<String>,
    },

    /// Return a unit to the state it was in before the run.
    RestoreUnit { unit: String, prior: PriorUnitState },

    /// Put the CPU scheduler back where it was.
    ///
    /// `None` unloads whatever gameready started, which hands scheduling back
    /// to the kernel's own scheduler immediately, with no reboot.
    RestoreScxScheduler { previous: Option<String> },

    /// Remove a directory, but only if nothing else put anything in it.
    RemoveDirIfEmpty { path: PathBuf },

    /// Recursively remove a directory tree gameready installed.
    ///
    /// Used for artifacts extracted from an archive, such as a Proton build,
    /// where the entire tree is one install unit and leaving it partially behind
    /// is worse than leaving it entirely behind.
    RemoveDirTree {
        path: PathBuf,
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },
}
