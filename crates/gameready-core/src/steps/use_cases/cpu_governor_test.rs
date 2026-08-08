use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};

const POLICY0_GOVERNOR: &str = "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor";
const POLICY1_GOVERNOR: &str = "/sys/devices/system/cpu/cpufreq/policy1/scaling_governor";

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

/// A laptop with two policies, both on powersave, both able to run performance,
/// no gamemode, and no governor daemon. Nothing else raises the clocks, so the
/// step applies.
fn laptop_on_powersave() -> MockRunner {
    MockRunner::new()
        .with_file(POLICY0_GOVERNOR, "powersave\n")
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors",
            "performance powersave\n",
        )
        .with_file(POLICY1_GOVERNOR, "powersave\n")
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy1/scaling_available_governors",
            "performance powersave\n",
        )
}

fn tuned_running(runner: MockRunner) -> MockRunner {
    runner
        .with_binary("systemctl")
        .answering("systemctl is-enabled tuned.service", "enabled")
        .answering("systemctl is-active tuned.service", "active")
}

fn probe(runner: &MockRunner) -> Probe {
    let facts = facts();
    let cx = CoreCx::new(&facts, runner);
    CpuGovernor.probe(&cx).expect("probed")
}

#[test]
fn a_machine_with_no_cpufreq_is_not_applicable() {
    // Normal in a virtual machine: there is no governor to set.
    assert!(matches!(
        probe(&MockRunner::new()),
        Probe::NotApplicable { .. }
    ));
}

#[test]
fn already_on_performance_is_already_applied() {
    let runner = laptop_on_powersave()
        .with_file(POLICY0_GOVERNOR, "performance\n")
        .with_file(POLICY1_GOVERNOR, "performance\n");
    assert!(matches!(probe(&runner), Probe::AlreadyApplied { .. }));
}

#[test]
fn hardware_without_a_performance_governor_is_not_applicable() {
    let runner = MockRunner::new()
        .with_file(POLICY0_GOVERNOR, "powersave\n")
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors",
            "powersave\n",
        );
    assert!(matches!(probe(&runner), Probe::NotApplicable { .. }));
}

#[test]
fn a_governor_daemon_is_a_conflict() {
    let runner = tuned_running(laptop_on_powersave());
    match probe(&runner) {
        Probe::Conflict { with, .. } => assert_eq!(with, "tuned.service"),
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn gamemode_present_is_already_applied() {
    let runner = laptop_on_powersave().with_binary("gamemoded");
    assert!(matches!(probe(&runner), Probe::AlreadyApplied { .. }));
}

#[test]
fn nothing_else_will_raise_it_so_it_applies() {
    assert!(matches!(probe(&laptop_on_powersave()), Probe::Applicable));
}

#[test]
fn a_conflicting_daemon_is_reported_ahead_of_gamemode() {
    // With a governor daemon live, gamemode's own raise is overwritten too, so
    // "gamemode has it" would be a lie. Row 4 must win over row 5.
    let runner = tuned_running(laptop_on_powersave()).with_binary("gamemoded");
    assert!(matches!(probe(&runner), Probe::Conflict { .. }));
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("journal")
}

#[test]
fn applying_sets_every_movable_policy_to_performance() {
    let dir = TempDir::new().expect("temp dir");
    let runner = laptop_on_powersave();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, CpuGovernor::id_const(), &runner, &mut journal);

    CpuGovernor.apply(&mut apply).expect("applied");

    assert_eq!(
        runner.file(POLICY0_GOVERNOR).as_deref(),
        Some("performance")
    );
    assert_eq!(
        runner.file(POLICY1_GOVERNOR).as_deref(),
        Some("performance")
    );
    assert!(
        runner.file(CPU_GOVERNOR_RULE).is_none(),
        "a live-only run must not write a boot rule"
    );
}

#[test]
fn verify_passes_once_every_policy_is_on_performance() {
    let dir = TempDir::new().expect("temp dir");
    let runner = laptop_on_powersave();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, CpuGovernor::id_const(), &runner, &mut journal);
    CpuGovernor.apply(&mut apply).expect("applied");

    let verification = CpuGovernor.verify(&cx).expect("verified");
    assert!(verification.passed(), "{verification:?}");
    assert!(verification.total_count() >= 2);
}

#[test]
fn rollback_puts_the_old_governor_back() {
    let dir = TempDir::new().expect("temp dir");
    let runner = laptop_on_powersave();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, CpuGovernor::id_const(), &runner, &mut journal);
    CpuGovernor.apply(&mut apply).expect("applied");

    let changes = apply.recorded().to_vec();
    CpuGovernor
        .rollback(&changes, &mut apply)
        .expect("rolled back");

    assert_eq!(runner.file(POLICY0_GOVERNOR).as_deref(), Some("powersave"));
    assert_eq!(runner.file(POLICY1_GOVERNOR).as_deref(), Some("powersave"));
}

#[test]
fn pinning_writes_the_boot_rule_and_rollback_removes_it() {
    let dir = TempDir::new().expect("temp dir");
    let runner = laptop_on_powersave();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_governor_pinned(true);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, CpuGovernor::id_const(), &runner, &mut journal);

    CpuGovernor.apply(&mut apply).expect("applied");
    assert!(
        runner.file(CPU_GOVERNOR_RULE).is_some(),
        "a pinned run writes the boot rule"
    );

    let changes = apply.recorded().to_vec();
    CpuGovernor
        .rollback(&changes, &mut apply)
        .expect("rolled back");
    assert!(
        runner.file(CPU_GOVERNOR_RULE).is_none(),
        "rollback removes the boot rule"
    );
}
