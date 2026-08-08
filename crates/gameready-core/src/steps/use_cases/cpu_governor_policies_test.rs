use super::*;
use crate::infra::exec::MockRunner;

fn two_policies() -> MockRunner {
    MockRunner::new()
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor",
            "powersave\n",
        )
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors",
            "performance powersave\n",
        )
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy1/scaling_governor",
            "performance\n",
        )
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy1/scaling_available_governors",
            "performance powersave\n",
        )
}

#[test]
fn it_reads_one_entry_per_policy() {
    let runner = two_policies();
    let policies = read_policies(&runner);
    assert_eq!(policies.len(), 2);
}

#[test]
fn a_machine_with_no_cpufreq_reads_nothing() {
    let policies = read_policies(&MockRunner::new());
    assert!(policies.is_empty());
}

#[test]
fn a_policy_only_moves_when_it_can_and_is_not_there_yet() {
    let runner = two_policies();
    let policies = read_policies(&runner);
    // policy0 is on powersave and offers performance, policy1 is already there.
    assert!(policies[0].needs_change());
    assert!(!policies[1].needs_change());
}

#[test]
fn hardware_that_does_not_offer_performance_never_moves() {
    let runner = MockRunner::new()
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor",
            "powersave\n",
        )
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors",
            "powersave\n",
        );
    let policies = read_policies(&runner);
    assert!(!policies[0].offers_performance());
    assert!(!policies[0].needs_change());
}

#[test]
fn a_live_governor_daemon_is_the_conflict() {
    let runner = MockRunner::new()
        .with_binary("systemctl")
        .answering("systemctl is-enabled tuned.service", "enabled")
        .answering("systemctl is-active tuned.service", "active");
    assert_eq!(conflicting_daemon(&runner), Some("tuned.service"));
}

#[test]
fn no_systemd_means_no_conflict() {
    // A container has no systemctl, and cannot be running either daemon.
    assert_eq!(conflicting_daemon(&MockRunner::new()), None);
}
