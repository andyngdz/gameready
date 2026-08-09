//! One reversible mutation, and how to reverse it.
//!
//! A `Change` is written to the journal and fsync'd *before* the mutation it
//! describes is performed. That ordering is the whole safety property: an
//! interrupt at any point leaves the system in a state that is a prefix of a
//! fully undoable sequence, never ahead of one.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::improvement::Privilege;

use super::undo::{PriorUnitState, Undo};

/// The privilege a record written before this field existed was made with.
///
/// Every step that predates it wrote under `/etc`, which is root's. Defaulting
/// to anything else would make an old journal undo itself without the privilege
/// it needs.
pub(crate) const fn assumed_root() -> Privilege {
    Privilege::Root
}

/// Hex digest of file contents.
///
/// Recorded when a file is written and re-checked before it is deleted, so
/// rollback can tell "unchanged since we wrote it" from "the user edited it
/// afterwards" and refuse to clobber the second.
#[must_use]
pub fn digest(contents: &str) -> String {
    Sha256::digest(contents.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

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
        /// How the write was made, so the undo is made the same way.
        ///
        /// Restoring a file in the user's home as root would leave it owned by
        /// root, and the program that owns it, such as Steam, would then fail
        /// to save its own settings.
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },

    /// A file gameready deleted, with its pre-image kept.
    FileRemoved {
        path: PathBuf,
        backup: PathBuf,
        mode: u32,
        #[serde(default = "assumed_root")]
        privilege: Privilege,
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

    /// A third-party package repository added to the system.
    ///
    /// Held as the spec its tooling accepts (`ppa:owner/name`) rather than as
    /// the files that were written, because the tool that adds one is the only
    /// thing that reliably knows which files those are, and it is also the
    /// thing that removes them.
    AptRepository { spec: String },

    /// A sched_ext CPU scheduler loaded at runtime.
    ///
    /// This one evaporates on reboot by itself, like
    /// [`SysctlRuntime`](Self::SysctlRuntime), and there is deliberately no
    /// paired file that makes it persist. The undo unloads it now so nobody has
    /// to reboot to get their old scheduler back.
    ScxScheduler {
        /// What was loaded before, if anything. `None` means the kernel was on
        /// its own scheduler and the undo is to hand it back.
        previous: Option<String>,
    },

    /// A directory gameready created.
    ///
    /// Carries the privilege it was made with, like every other path record: a
    /// directory in the user's own home must be removed as the user, and a run
    /// that only made one must not ask for a password to undo itself.
    DirCreated {
        path: PathBuf,
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },

    /// A directory tree gameready installed, such as a Proton build extracted
    /// from a tarball. Unlike [`DirCreated`](Self::DirCreated), the undo is a
    /// recursive delete rather than an empty-only remove, because the directory
    /// was populated by the install itself and keeping it would leave behind
    /// exactly the artifact the user asked to undo.
    DirTreeInstalled {
        path: PathBuf,
        #[serde(default = "assumed_root")]
        privilege: Privilege,
    },
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
                privilege,
            } => match (existed, backup) {
                (true, Some(backup)) => Undo::RestoreFile {
                    path: path.clone(),
                    from: backup.clone(),
                    mode: *mode,
                    privilege: *privilege,
                },
                _ => Undo::DeleteFile {
                    path: path.clone(),
                    expect_sha256: sha256_after.clone(),
                    privilege: *privilege,
                },
            },

            Self::FileRemoved {
                path,
                backup,
                mode,
                privilege,
            } => Undo::RestoreFile {
                path: path.clone(),
                from: backup.clone(),
                mode: *mode,
                privilege: *privilege,
            },

            Self::SysctlRuntime { key, previous } => Undo::SetSysctl {
                key: key.clone(),
                value: previous.clone(),
            },

            Self::SysfsWrite { path, previous } => Undo::WriteSysfs {
                path: path.clone(),
                value: previous.clone(),
            },

            Self::PackagesInstalled {
                manager,
                newly_installed,
                ..
            } => Undo::ReportPackages {
                manager: manager.clone(),
                installed: newly_installed.clone(),
            },

            Self::SystemdUnit {
                unit, was_enabled, ..
            } => Undo::RestoreUnit {
                unit: unit.clone(),
                prior: if *was_enabled {
                    PriorUnitState::WasEnabled
                } else {
                    PriorUnitState::WasDisabled
                },
            },

            Self::AptRepository { spec } => Undo::RemoveAptRepository { spec: spec.clone() },

            Self::ScxScheduler { previous } => Undo::RestoreScxScheduler {
                previous: previous.clone(),
            },

            Self::DirCreated { path, privilege } => Undo::RemoveDirIfEmpty {
                path: path.clone(),
                privilege: *privilege,
            },

            Self::DirTreeInstalled { path, privilege } => Undo::RemoveDirTree {
                path: path.clone(),
                privilege: *privilege,
            },
        }
    }
}

#[cfg(test)]
#[path = "change_test.rs"]
mod change_test;
