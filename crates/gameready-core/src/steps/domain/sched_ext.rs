//! What the kernel is scheduling with, and the commands that change it.

use crate::exec::Cmd;
use crate::improvement::PackageSpec;
use crate::steps::constants::SCXCTL_BIN;

/// Installed sizes, read from the Arch package index on 2026-08-07.
///
/// `scx-scheds` is large because it carries seventeen scheduler binaries, each
/// with its BPF object. The user sees this number before agreeing, which is the
/// whole reason it is here and not rounded down.
const SCX_SCHEDS_BYTES: u64 = 178_850_464;
const SCX_TOOLS_BYTES: u64 = 7_777_439;

/// The scheduler binaries.
///
/// Ubuntu is the odd one out: `ppa:arighi/sched-ext` ships everything in a
/// single `scx` package rather than splitting binaries from tooling, so the
/// tooling spec below has no apt name and this one covers both there.
pub const SCX_SCHEDS: PackageSpec = PackageSpec {
    pacman: Some("scx-scheds"),
    apt: Some("scx"),
    dnf: Some("scx-scheds"),
    approx_bytes: SCX_SCHEDS_BYTES,
};

/// `scx_loader`, `scxctl`, and the D-Bus and polkit files that let a desktop
/// user switch schedulers without a root shell.
///
/// A separate package on Arch and on the Fedora COPR. Loading a scheduler needs
/// both, which is the mistake worth not repeating: `scx-scheds` alone installs
/// no `scxctl` and no loader service.
pub const SCX_TOOLS: PackageSpec = PackageSpec {
    pacman: Some("scx-tools"),
    apt: None,
    dnf: Some("scx-tools"),
    approx_bytes: SCX_TOOLS_BYTES,
};

/// The command that loads `scheduler` in the loader's gaming mode.
#[must_use]
pub fn load_scheduler(scheduler: &str) -> Cmd {
    Cmd::root(SCXCTL_BIN)
        .arg("start")
        .arg("-s")
        .arg(scheduler)
        .arg("-m")
        .arg("gaming")
}

/// The command that hands scheduling back to `previous`, or to the kernel.
///
/// Shared with the rollback engine so the undo the journal promises and the
/// undo a failed apply performs cannot drift into two different commands.
#[must_use]
pub fn restore_scheduler(previous: Option<&str>) -> Cmd {
    match previous {
        Some(scheduler) => Cmd::root(SCXCTL_BIN).arg("switch").arg("-s").arg(scheduler),
        None => Cmd::root(SCXCTL_BIN).arg("stop"),
    }
}

/// Whether this kernel can run a sched_ext scheduler, and what it is running.
///
/// Three states rather than a pair of booleans, because the step answers
/// differently to each: a kernel without sched_ext can never run this step, an
/// idle one is ready for it, and one already running something is a machine
/// where gameready is not the only owner of the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedExt {
    /// The kernel was built without sched_ext. Nothing can be loaded.
    Unsupported,

    /// Built in, with nothing attached. The kernel is scheduling on its own.
    Idle,

    /// A scheduler is attached and making the decisions.
    Running {
        /// Which one, when the kernel names it.
        ///
        /// `None` means the kernel says something is attached but does not
        /// expose the name where it is documented to be. That is a real
        /// unknown, not a fourth state: something else owns the scheduler
        /// either way, and the step has to keep its hands off either way.
        scheduler: Option<String>,
    },
}

impl SchedExt {
    /// Whether `scheduler` is the one currently attached.
    #[must_use]
    pub fn is_running(&self, scheduler: &str) -> bool {
        matches!(self, Self::Running { scheduler: Some(running) } if running == scheduler)
    }

    /// How to name whatever is attached, for a message a user reads.
    #[must_use]
    pub fn describe(&self) -> &str {
        match self {
            Self::Unsupported => "no sched_ext support",
            Self::Idle => "the kernel's own scheduler",
            Self::Running { scheduler: None } => "an unnamed sched_ext scheduler",
            Self::Running {
                scheduler: Some(running),
            } => running,
        }
    }

    /// The attached scheduler, for the journal record that has to put it back.
    #[must_use]
    pub fn previous(&self) -> Option<String> {
        match self {
            Self::Unsupported | Self::Idle => None,
            Self::Running { scheduler } => scheduler.clone(),
        }
    }
}

#[cfg(test)]
#[path = "sched_ext_test.rs"]
mod sched_ext_test;
