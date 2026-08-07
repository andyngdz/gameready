//! How a given system loads a sched_ext scheduler.
//!
//! There are two mechanisms in the wild and they share nothing. Arch and the
//! Fedora COPR ship `scx-tools`, which brings `scx_loader` and the `scxctl`
//! client that switches schedulers while the machine runs. The Ubuntu PPA ships
//! a single `scx` package with the scheduler binaries and a systemd unit, and
//! no loader at all.

use std::path::{Path, PathBuf};

use crate::exec::Cmd;
use crate::facts::PackageManagerKind;
use crate::improvement::{ApplyCx, CoreCx, Privilege, StepError};
use crate::journal::{digest, Change};
use crate::steps::constants::{
    LAVD_SCHEDULER, SCXCTL_BIN, SCX_LAVD_BIN, SCX_SCHEDULER_OVERRIDE, SCX_SERVICE_NAME,
    SCX_UNIT_DROPIN, SCX_UNIT_PATH,
};
use crate::steps::domain::load_scheduler;
use crate::systemd::{ENABLE, NOW, SYSTEMCTL};

/// Which mechanism this system uses to put a scheduler in charge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Loader {
    /// `scx_loader` owns the scheduler and `scxctl` switches it at runtime.
    /// Nothing is written, and the choice is gone after a reboot.
    Scxctl,

    /// The distro's own `scx.service` runs the scheduler. A drop-in points it
    /// at scx_lavd, so the choice comes back on the next boot too.
    Unit,
}

impl Loader {
    /// What this system would use, from what is installed and what would be.
    ///
    /// Falls back to the family's packaging when neither is installed yet,
    /// because the plan screen has to say what will happen before anything is
    /// fetched.
    pub fn detect(cx: &CoreCx<'_>) -> Self {
        if cx.runner.which(SCXCTL_BIN).is_some() {
            return Self::Scxctl;
        }
        if cx.runner.path_exists(Path::new(SCX_UNIT_PATH)) {
            return Self::Unit;
        }
        match cx.facts.distro.package_manager() {
            PackageManagerKind::Apt => Self::Unit,
            PackageManagerKind::Pacman | PackageManagerKind::Dnf => Self::Scxctl,
        }
    }

    /// Whether the choice is still there after a reboot.
    ///
    /// Said out loud on the plan screen, because the two mechanisms differ on
    /// exactly the property a user needs to know before agreeing.
    #[must_use]
    pub const fn survives_reboot(self) -> bool {
        matches!(self, Self::Unit)
    }

    /// The command line the plan screen shows.
    #[must_use]
    pub fn describe(self) -> String {
        match self {
            Self::Scxctl => load_scheduler(LAVD_SCHEDULER).to_string(),
            Self::Unit => Cmd::root(SYSTEMCTL)
                .arg(ENABLE)
                .arg(NOW)
                .arg(SCX_SERVICE_NAME)
                .to_string(),
        }
    }

    /// Puts scx_lavd in charge.
    pub fn load(self, cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
        match self {
            Self::Scxctl => load_with_scxctl(cx),
            Self::Unit => load_with_unit(cx),
        }
    }
}

/// Runs the scheduler now, through the loader daemon.
fn load_with_scxctl(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
    let previous = crate::steps::use_cases::scx_state::read_sched_ext(cx.reader()).previous();
    cx.mutate(Change::ScxScheduler { previous }, |runner| {
        let load = load_scheduler(LAVD_SCHEDULER);
        runner.run(&load).map(|_| ()).map_err(StepError::Exec)
    })
}

/// Points the distro's unit at scx_lavd and starts it.
///
/// The drop-in goes down before the unit starts, so a run interrupted between
/// the two leaves a machine that is correct on the next boot rather than
/// correct now and wrong later.
fn load_with_unit(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
    let dropin = PathBuf::from(SCX_UNIT_DROPIN);
    let contents = dropin_contents(cx.run());

    cx.mutate(
        Change::FileWritten {
            path: dropin.clone(),
            existed: false,
            backup: None,
            sha256_after: digest(&contents),
            mode: 0o644,
            privilege: Privilege::Root,
        },
        |runner| {
            runner
                .write_file(&dropin, &contents, Privilege::Root)
                .map_err(StepError::Exec)
        },
    )?;

    cx.mutate(
        Change::SystemdUnit {
            unit: SCX_SERVICE_NAME.to_owned(),
            was_enabled: false,
            was_active: false,
        },
        |runner| {
            let start = Cmd::root(SYSTEMCTL)
                .arg(ENABLE)
                .arg(NOW)
                .arg(SCX_SERVICE_NAME);
            runner.run(&start).map(|_| ()).map_err(StepError::Exec)
        },
    )
}

/// The drop-in body, carrying the marker `doctor` finds it by.
fn dropin_contents(run: crate::journal::RunId) -> String {
    format!(
        "{header}\n[Service]\nEnvironment={SCX_SCHEDULER_OVERRIDE}={SCX_LAVD_BIN}\n",
        header = crate::steps::constants::managed_header(
            crate::steps::use_cases::scx_lavd::ScxLavd::id_const(),
            run
        ),
    )
}

#[cfg(test)]
#[path = "scx_lavd_loader_test.rs"]
mod scx_lavd_loader_test;
