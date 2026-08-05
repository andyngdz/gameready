//! One reversible mutation, and how to reverse it.
//!
//! A `Change` is written to the journal and fsync'd *before* the mutation it
//! describes is performed. That ordering is the whole safety property: an
//! interrupt at any point leaves the system in a state that is a prefix of a
//! fully undoable sequence, never ahead of one.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A mutation gameready made, carrying enough prior state to undo it.
///
/// Every variant records what was there before, not just what was put there.
/// Recording only the new value would make rollback a guess.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    /// A file gameready created. `/etc` files are only ever created, never
    /// edited in place, so `existed: false` is the normal case and the undo is
    /// a delete. The `existed: true` path exists for files outside `/etc`, such
    /// as Steam config, where a pre-image is kept under `backups/`.
    FileWritten {
        path: PathBuf,
        existed: bool,
        /// Pre-image location, present only when `existed` is true.
        backup: Option<PathBuf>,
        /// Digest of what gameready wrote, so rollback can tell "unchanged
        /// since we wrote it" from "the user edited it afterwards".
        sha256_after: String,
        mode: u32,
    },

    /// A file gameready deleted, with its pre-image kept.
    FileRemoved {
        path: PathBuf,
        backup: PathBuf,
        mode: u32,
    },

    /// A kernel parameter set at runtime. Evaporates on reboot on its own; the
    /// paired `FileWritten` under `/etc/sysctl.d` is what makes it persist.
    SysctlRuntime { key: String, previous: String },

    /// A sysfs attribute written, such as a block device queue scheduler.
    SysfsWrite { path: PathBuf, previous: String },

    /// Packages installed. `newly_installed` is the subset that was not already
    /// present, which is the only part removal should ever consider.
    PackagesInstalled {
        manager: String,
        requested: Vec<String>,
        newly_installed: Vec<String>,
    },

    /// A systemd unit whose enabled or active state was changed.
    SystemdUnit {
        unit: String,
        was_enabled: bool,
        was_active: bool,
    },

    /// A directory gameready created.
    DirCreated { path: PathBuf },
}

impl Change {
    /// What undoing this change requires.
    ///
    /// Returns the operation rather than performing it, so rollback can be
    /// previewed with `--dry-run` and so the inverse is unit-testable without a
    /// system to mutate.
    #[must_use]
    pub fn inverse(&self) -> Undo {
        match self {
            Self::FileWritten {
                path,
                existed,
                backup,
                sha256_after,
                mode,
            } => match (existed, backup) {
                (true, Some(backup)) => Undo::RestoreFile {
                    path: path.clone(),
                    from: backup.clone(),
                    mode: *mode,
                },
                // Created by us, so the inverse is removal. The digest lets the
                // caller refuse to delete a file the user edited afterwards.
                _ => Undo::DeleteFile {
                    path: path.clone(),
                    expect_sha256: sha256_after.clone(),
                },
            },

            Self::FileRemoved { path, backup, mode } => Undo::RestoreFile {
                path: path.clone(),
                from: backup.clone(),
                mode: *mode,
            },

            Self::SysctlRuntime { key, previous } => Undo::SetSysctl {
                key: key.clone(),
                value: previous.clone(),
            },

            Self::SysfsWrite { path, previous } => Undo::WriteSysfs {
                path: path.clone(),
                value: previous.clone(),
            },

            // Uninstalling is not the inverse of installing: dependency
            // cascades, leftover config, and other users relying on the package
            // all make removal a different operation with different blast
            // radius. Default is to report and leave; `--purge-packages` opts
            // into the removal explicitly.
            Self::PackagesInstalled {
                manager,
                newly_installed,
                ..
            } => Undo::ReportPackages {
                manager: manager.clone(),
                installed: newly_installed.clone(),
            },

            Self::SystemdUnit {
                unit,
                was_enabled,
                was_active,
            } => Undo::RestoreUnit {
                unit: unit.clone(),
                enabled: *was_enabled,
                active: *was_active,
            },

            Self::DirCreated { path } => Undo::RemoveDirIfEmpty { path: path.clone() },
        }
    }
}

/// The operation that reverses a [`Change`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "undo", rename_all = "snake_case")]
pub enum Undo {
    /// Delete a file gameready created. Refuses if the file no longer matches
    /// `expect_sha256`, because the user edited it and clobbering their edit is
    /// worse than leaving the file behind.
    DeleteFile {
        path: PathBuf,
        expect_sha256: String,
    },

    /// Put a pre-image back, restoring the recorded mode.
    RestoreFile {
        path: PathBuf,
        from: PathBuf,
        mode: u32,
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

    /// Return a unit to its prior enabled and active state.
    RestoreUnit {
        unit: String,
        enabled: bool,
        active: bool,
    },

    /// Remove a directory, but only if nothing else put anything in it.
    RemoveDirIfEmpty { path: PathBuf },
}

#[cfg(test)]
#[path = "change_test.rs"]
mod change_test;
