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
use crate::steps::domain::{load_scheduler, restore_scheduler};
use crate::steps::use_cases::scx_state::read_sched_ext;
use crate::systemd::{unit_state, UnitState, ENABLE, NOW, STOP, SYSTEMCTL};

/// The command that would hand the CPU back, when this run can do it cleanly.
///
/// `None` when gameready has no way to stop what is running: a scheduler
/// somebody started by hand, or from a unit gameready does not manage, has an
/// owner this run cannot put back, and guessing at it would be worse than
/// saying so. This is what turns a conflict into a question a run can ask.
pub(super) fn takeover_stop(cx: &CoreCx<'_>) -> Option<String> {
    if cx.runner.which(SCXCTL_BIN).is_some() {
        return Some(restore_scheduler(None).to_string());
    }
    matches!(
        unit_state(cx.runner, SCX_SERVICE_NAME),
        Ok(UnitState::Running)
    )
    .then(|| {
        Cmd::root(SYSTEMCTL)
            .arg(STOP)
            .arg(SCX_SERVICE_NAME)
            .to_string()
    })
}

/// How long an attaching scheduler may take, and how often the load asks.
///
/// `systemctl enable --now` returns once the unit's wrapper shell is up, but
/// the BPF program it execs attaches a couple of seconds later. The window
/// has to cover a scheduler loaded from a cold cache, which is the slowest
/// case.
const ATTACH_WINDOW: std::time::Duration = std::time::Duration::from_secs(15);
const ATTACH_POLL: std::time::Duration = std::time::Duration::from_millis(250);

/// Waits until the kernel reports the scheduler this load started.
///
/// Without the wait a step would return the moment its start command
/// succeeded and then fail verification against a kernel that had not caught
/// up, undoing a load that was about to land.
fn wait_for_attach(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
    let deadline = std::time::Instant::now() + ATTACH_WINDOW;
    loop {
        if read_sched_ext(cx.reader()).is_running(LAVD_SCHEDULER) {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(StepError::StartupTimeout {
                what: format!("scx_{LAVD_SCHEDULER}"),
                window: ATTACH_WINDOW.as_secs(),
            });
        }
        std::thread::sleep(ATTACH_POLL);
    }
}

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
///
/// When a different scheduler is already attached, the loader hands it back to
/// the kernel first. The previous scheduler is still read before that stop and
/// recorded with the load, so the undo is a switch back to it rather than a
/// stop.
fn load_with_scxctl(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
    let previous = read_sched_ext(cx.reader()).previous();
    if previous.is_some() {
        let stop = restore_scheduler(None);
        cx.reader().run(&stop).map_err(StepError::Exec)?;
    }
    cx.mutate(Change::ScxScheduler { previous }, |runner| {
        let load = load_scheduler(LAVD_SCHEDULER);
        runner.run(&load).map(|_| ()).map_err(StepError::Exec)
    })?;
    wait_for_attach(cx)
}

/// Points the distro's unit at scx_lavd and starts it.
///
/// The drop-in goes down before the unit starts, so a run interrupted between
/// the two leaves a machine that is correct on the next boot rather than
/// correct now and wrong later. When the unit is what is running the current
/// scheduler, the takeover stops it first.
fn load_with_unit(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
    let prior = unit_state(cx.reader(), SCX_SERVICE_NAME).unwrap_or(UnitState::Dormant);
    if prior == UnitState::Running {
        hand_the_unit_over(cx, prior)?;
    }
    write_the_dropin(cx)?;
    start_the_unit(cx, prior)?;
    wait_for_attach(cx)
}

/// Stops the unit a takeover is about to re-point, recording its handover
/// first: the journal's newest-first undo then removes the drop-in before it
/// restarts the unit, so the unit comes back running its own scheduler.
fn hand_the_unit_over(cx: &mut ApplyCx<'_, CoreCx<'_>>, prior: UnitState) -> Result<(), StepError> {
    cx.record(Change::SystemdUnit {
        unit: SCX_SERVICE_NAME.to_owned(),
        was_enabled: prior.is_live(),
        was_active: true,
    })?;
    let stop = Cmd::root(SYSTEMCTL).arg(STOP).arg(SCX_SERVICE_NAME);
    cx.reader().run(&stop).map(|_| ()).map_err(StepError::Exec)
}

/// Writes the drop-in that points the unit at scx_lavd.
fn write_the_dropin(cx: &mut ApplyCx<'_, CoreCx<'_>>) -> Result<(), StepError> {
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
    )
}

/// Starts the unit now and at boot, recording what it changes.
///
/// A takeover already recorded the unit's handover before the drop-in went
/// down, so for one this is only the start.
fn start_the_unit(cx: &mut ApplyCx<'_, CoreCx<'_>>, prior: UnitState) -> Result<(), StepError> {
    let start = Cmd::root(SYSTEMCTL)
        .arg(ENABLE)
        .arg(NOW)
        .arg(SCX_SERVICE_NAME);
    if prior == UnitState::Running {
        return cx.reader().run(&start).map(|_| ()).map_err(StepError::Exec);
    }
    cx.mutate(
        Change::SystemdUnit {
            unit: SCX_SERVICE_NAME.to_owned(),
            was_enabled: prior.is_live(),
            was_active: false,
        },
        |runner| runner.run(&start).map(|_| ()).map_err(StepError::Exec),
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
