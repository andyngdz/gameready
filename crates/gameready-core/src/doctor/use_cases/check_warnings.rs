//! Scanning for settings gameready refuses to apply and that the user should
//! know about if they applied them manually.

use crate::doctor::domain::Warning;
use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::KernelVersion;

const CMDLINE_PATH: &str = "/proc/cmdline";
const MITIGATIONS_OFF: &str = "mitigations=off";

const SWAPPINESS_PATH: &str = "/proc/sys/vm/swappiness";
const SWAPPINESS_BAD: &str = "1";

/// Kernel 6.6 removed CFS tunables; writing them to sysctl.d is a silent
/// no-op that looks like a tuning step succeeded.
const EEVDF_CUTOFF: KernelVersion = KernelVersion::new(6, 6, 0);

const DEAD_SYSCTLS: &[&str] = &[
    "kernel.sched_latency_ns",
    "kernel.sched_min_granularity_ns",
    "kernel.sched_wakeup_granularity_ns",
];

/// Runs every warning check and returns what was found.
pub fn check_warnings(facts: &SystemFacts, runner: &dyn CommandRunner) -> Vec<Warning> {
    let mut warnings = Vec::new();
    check_mitigations(runner, &mut warnings);
    check_swappiness(runner, &mut warnings);
    check_dead_sysctls(facts, runner, &mut warnings);
    warnings
}

fn check_mitigations(runner: &dyn CommandRunner, warnings: &mut Vec<Warning>) {
    let cmdline = match runner.read_to_string(CMDLINE_PATH.as_ref()) {
        Ok(content) => content,
        Err(_) => return,
    };

    if cmdline
        .split_whitespace()
        .any(|token| token == MITIGATIONS_OFF)
    {
        warnings.push(Warning::new(
            "mitigations=off is set in the kernel command line",
            "This disables Spectre and Meltdown protections for roughly 0-3% FPS \
             on CPUs from 2020 onward. The security cost is real; the performance \
             gain is not measurable in most games.",
            "Remove mitigations=off from your bootloader config. gameready will \
             never set this.",
        ));
    }
}

fn check_swappiness(runner: &dyn CommandRunner, warnings: &mut Vec<Warning>) {
    let swappiness = match runner.read_to_string(SWAPPINESS_PATH.as_ref()) {
        Ok(content) => content,
        Err(_) => return,
    };

    if swappiness.trim() == SWAPPINESS_BAD {
        warnings.push(Warning::new(
            "vm.swappiness is set to 1",
            "This was advice from the HDD era. On modern systems with zram swap, \
             swappiness=1 forces the kernel to evict file cache instead of \
             compressing anonymous pages, which hurts performance under memory \
             pressure. The kernel default of 60 is correct for disk swap; 180 is \
             better when zram is the primary swap.",
            "Remove the swappiness override from /etc/sysctl.d/ or \
             /etc/sysctl.conf. gameready sets swappiness only when zram is the \
             primary swap, and never to 1.",
        ));
    }
}

fn check_dead_sysctls(
    facts: &SystemFacts,
    runner: &dyn CommandRunner,
    warnings: &mut Vec<Warning>,
) {
    if facts.kernel < EEVDF_CUTOFF {
        return;
    }

    let sysctl_conf = runner
        .read_to_string("/etc/sysctl.conf".as_ref())
        .unwrap_or_default();

    for key in DEAD_SYSCTLS {
        let proc_path = format!("/proc/sys/{}", key.replace('.', "/"));
        if runner.path_exists(proc_path.as_ref()) {
            continue;
        }

        let configured = sysctl_conf.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && trimmed.starts_with(key)
        });

        if configured {
            warnings.push(Warning::new(
                format!("{key} is configured but the knob no longer exists"),
                format!(
                    "Kernel 6.6 replaced CFS with EEVDF and removed {key}. \
                     The line in sysctl.conf is silently ignored."
                ),
                format!("Remove the {key} line from /etc/sysctl.conf."),
            ));
        }
    }
}

#[cfg(test)]
#[path = "check_warnings_test.rs"]
mod check_warnings_test;
