//! What a step would do, rendered before the user agrees to it.

use serde::{Deserialize, Serialize};

use crate::improvement::domain::identity::ImprovementId;

/// One step's contribution to the confirmation screen.
///
/// Built by `plan()`, which must not mutate anything. Whatever appears here is
/// the promise the step makes; `apply()` doing more than this is a bug the user
/// has no way to catch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepPlan {
    /// Which step this belongs to.
    pub step: ImprovementId,

    /// One line, in the terms a user recognises.
    /// "vm.max_map_count 1048576 -> 2147483642"
    pub summary: String,

    /// Every distinct change, for `--verbose` and for `explain`.
    pub actions: Vec<PlannedAction>,
}

impl StepPlan {
    /// A plan with a summary line and no actions listed yet.
    #[must_use]
    pub fn new(step: ImprovementId, summary: impl Into<String>) -> Self {
        Self {
            step,
            summary: summary.into(),
            actions: Vec::new(),
        }
    }

    /// Adds one change to the detail list.
    #[must_use]
    pub fn action(mut self, action: PlannedAction) -> Self {
        self.actions.push(action);
        self
    }
}

/// One package a step will install, with the text needed to ask about it.
///
/// A name on its own cannot be agreed to: someone who has never heard of
/// mangohud has nothing to decide with. The step is the only place that knows
/// what the package is and why this run wants it, so it says so here rather
/// than leaving the screen to invent a reason from the step's title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedPackage {
    /// The name this distro's tooling uses.
    pub name: String,

    /// What it is, in one sentence, for someone who has not heard of it.
    pub what: String,

    /// Why this run wants it, in one sentence.
    pub why: String,

    /// Rough download size in bytes.
    pub approx_bytes: u64,
}

/// A single change a step intends to make.
///
/// Deliberately concrete rather than a free-text string: the confirmation
/// screen, the `--json` output, and the rollback preview all read these, and a
/// string would force each of them to parse prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlannedAction {
    /// Create a file that does not exist. gameready only ever creates new files
    /// under `/etc`, never edits existing ones, so that losing the journal
    /// still leaves every change identifiable and removable.
    CreateFile { path: String, contents: String },

    /// Set a kernel parameter at runtime.
    SetSysctl {
        key: String,
        from: String,
        to: String,
    },

    /// Write to a sysfs attribute, such as a block device queue scheduler.
    WriteSysfs {
        path: String,
        from: String,
        to: String,
    },

    /// Install packages through the system package manager.
    InstallPackages {
        /// What will be fetched.
        packages: Vec<PlannedPackage>,

        /// Packages this step wanted that the machine already has. Carried so a
        /// screen can show that a step called "install gamemode and mangohud"
        /// is only going to fetch one of them.
        already_present: Vec<String>,
    },

    /// Enable and start a systemd unit.
    EnableUnit { unit: String },

    /// Anything else, shown as the exact command line that will run.
    RunCommand { display: String },
}
