//! The operations that reverse a [`Change`](super::Change).

use std::path::{Path, PathBuf};

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

    /// Take a third-party repository back off the system.
    RemoveAptRepository { spec: String },

    /// Put the CPU scheduler back where it was.
    ///
    /// `None` unloads whatever gameready started, which hands scheduling back
    /// to the kernel's own scheduler immediately, with no reboot.
    RestoreScxScheduler { previous: Option<String> },

    /// Remove a directory, but only if nothing else put anything in it.
    RemoveDirIfEmpty {
        path: PathBuf,
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },

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

impl Undo {
    /// A short human name for what this operation puts back, shown as the
    /// subject of a rollback row before the note. Taken from the operation
    /// rather than the step, because the row is about the thing restored, not
    /// the tuning that touched it.
    #[must_use]
    pub fn subject(&self) -> String {
        match self {
            Self::SetSysctl { key, .. } => key.clone(),
            Self::WriteSysfs { path, .. } => scheduler_subject(path),
            Self::RestoreScxScheduler { .. } => "CPU scheduler".to_owned(),
            Self::RestoreUnit { unit, .. } => unit.clone(),
            Self::RemoveAptRepository { spec } => spec.clone(),
            Self::ReportPackages { .. } => "packages".to_owned(),
            Self::RestoreFile { path, .. }
            | Self::DeleteFile { path, .. }
            | Self::RemoveDirIfEmpty { path, .. }
            | Self::RemoveDirTree { path, .. } => file_name(path),
        }
    }

    /// What this operation has to be run as.
    ///
    /// The path operations carry the privilege the change was made with, so
    /// undoing something in the user's own home is a user's job and a run that
    /// only touched their home never asks for a password. Everything else
    /// touches the system by definition: `/proc/sys`, `/sys`, a systemd unit,
    /// an apt repository.
    #[must_use]
    pub const fn privilege(&self) -> Privilege {
        match self {
            Self::RestoreFile { privilege, .. }
            | Self::DeleteFile { privilege, .. }
            | Self::RemoveDirIfEmpty { privilege, .. }
            | Self::RemoveDirTree { privilege, .. } => *privilege,
            Self::SetSysctl { .. }
            | Self::WriteSysfs { .. }
            | Self::RestoreScxScheduler { .. }
            | Self::RestoreUnit { .. }
            | Self::RemoveAptRepository { .. } => Privilege::Root,
            // Reporting packages changes nothing. Removing them does, and that
            // is the caller's policy rather than a property of this record.
            Self::ReportPackages { .. } => Privilege::User,
        }
    }
}

/// Names a sysfs write by its block device, since the only attribute gameready
/// writes there is the per-disk I/O scheduler.
fn scheduler_subject(path: &Path) -> String {
    path.components()
        .skip_while(|component| component.as_os_str() != "block")
        .nth(1)
        .map_or_else(
            || file_name(path),
            |device| format!("I/O scheduler {}", device.as_os_str().to_string_lossy()),
        )
}

/// The last path component, for naming a file or directory undo.
fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
