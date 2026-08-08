//! Reading the CPU frequency-scaling policies and their governors.

use std::path::{Path, PathBuf};

use itertools::Itertools as _;

use crate::exec::CommandRunner;
use crate::steps::domain::GOVERNOR_DAEMONS;
use crate::systemd::{unit_state, UnitState};

/// Where the kernel lists one directory per frequency-scaling policy.
///
/// The governor is per policy, not per core: a big.LITTLE machine has one
/// policy for its performance cores and one for its efficiency cores, and both
/// have to be set or a reboot silently leaves half the machine behind.
pub(crate) const CPUFREQ_DIR: &str = "/sys/devices/system/cpu/cpufreq";

/// A policy's current governor, relative to its `cpufreq` policy directory.
pub(crate) const SCALING_GOVERNOR_FILE: &str = "scaling_governor";

/// The governors a policy's hardware can offer, space separated, relative to
/// its `cpufreq` policy directory.
///
/// Read so the step can tell "not on performance yet" apart from "this hardware
/// has no performance governor at all", which is a permanent property of some
/// laptops and not something installing anything will fix.
pub(crate) const SCALING_AVAILABLE_GOVERNORS_FILE: &str = "scaling_available_governors";

/// The governor gameready pins, the one that holds the clocks up.
pub(crate) const PERFORMANCE_GOVERNOR: &str = "performance";

/// The udev rule that re-pins the governor on every boot.
///
/// Written only when the user asks to keep the change across reboots; the
/// default is a live write that lasts until the next boot and leaves no file at
/// all. Its own file, never an edit, like every other `/etc` file gameready
/// writes.
pub(crate) const CPU_GOVERNOR_RULE: &str = "/etc/udev/rules.d/60-gameready-cpu-governor.rules";

/// One frequency-scaling policy: where its governor lives, what it is set to
/// now, and what its hardware can offer.
pub(super) struct GovernorPolicy {
    pub(super) name: String,
    pub(super) governor_path: PathBuf,
    pub(super) current: String,
    available: Vec<String>,
}

impl GovernorPolicy {
    /// Whether this policy's hardware offers the performance governor at all.
    pub(super) fn offers_performance(&self) -> bool {
        self.available
            .iter()
            .any(|governor| governor == PERFORMANCE_GOVERNOR)
    }

    /// Whether this policy is already running the performance governor.
    pub(super) fn is_performance(&self) -> bool {
        self.current == PERFORMANCE_GOVERNOR
    }

    /// Whether pinning would move this policy: it can take performance and is
    /// not already there.
    pub(super) fn needs_change(&self) -> bool {
        self.offers_performance() && !self.is_performance()
    }
}

/// Reads every frequency-scaling policy under `cpufreq` with its governor.
///
/// A machine with no `cpufreq` at all, normal in a virtual machine, lists
/// nothing rather than failing: the directory is simply not there, and the step
/// reads that as "nothing to pin" rather than an error. A single policy whose
/// files cannot be read is skipped, the way one odd disk does not stop the I/O
/// scheduler from tuning the rest.
pub(super) fn read_policies(runner: &dyn CommandRunner) -> Vec<GovernorPolicy> {
    let mut policies = Vec::new();
    for entry in runner.read_dir(Path::new(CPUFREQ_DIR)).unwrap_or_default() {
        let Some(name) = entry.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("policy") {
            continue;
        }
        let governor_path = entry.join(SCALING_GOVERNOR_FILE);
        let Some(current) = read_trimmed(runner, &governor_path) else {
            continue;
        };
        let Some(available) = read_trimmed(runner, &entry.join(SCALING_AVAILABLE_GOVERNORS_FILE))
        else {
            continue;
        };
        policies.push(GovernorPolicy {
            name: name.to_owned(),
            governor_path,
            current,
            available: available.split_whitespace().map(str::to_owned).collect(),
        });
    }
    policies
}

/// One line naming what pinning would move, for the plan screen.
pub(super) fn summary(policies: &[GovernorPolicy]) -> String {
    let changing = policies
        .iter()
        .filter(|policy| policy.needs_change())
        .map(|policy| {
            format!(
                "{} {} -> {PERFORMANCE_GOVERNOR}",
                policy.name, policy.current
            )
        })
        .join(", ");
    if changing.is_empty() {
        "CPU governor already on performance".to_owned()
    } else {
        format!("CPU governor: {changing}")
    }
}

/// The first governor-owning daemon that is live, if any.
///
/// A machine with no systemd cannot be running either of them, so a query that
/// cannot be answered reads as "no conflict" rather than stopping the step.
pub(super) fn conflicting_daemon(runner: &dyn CommandRunner) -> Option<&'static str> {
    GOVERNOR_DAEMONS.into_iter().find(|unit| {
        unit_state(runner, unit)
            .map(UnitState::is_live)
            .unwrap_or(false)
    })
}

/// Reads a sysfs attribute, trimmed, treating an empty read as nothing there.
fn read_trimmed(runner: &dyn CommandRunner, path: &Path) -> Option<String> {
    runner
        .read_to_string(path)
        .ok()
        .map(|raw| raw.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "cpu_governor_policies_test.rs"]
mod cpu_governor_policies_test;
