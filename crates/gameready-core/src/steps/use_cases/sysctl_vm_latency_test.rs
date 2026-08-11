use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::MANAGED_HEADER;
use crate::steps::domain::VM_LATENCY_KNOBS;

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

/// A kernel exposing every parameter, all holding the given value.
fn system_at(value: &str) -> MockRunner {
    let mut runner = MockRunner::new();
    for knob in VM_LATENCY_KNOBS {
        runner = runner.with_file(
            knob.runtime_path().to_string_lossy().as_ref(),
            format!("{value}\n"),
        );
    }
    runner
}

/// A kernel already holding every target.
fn system_tuned() -> MockRunner {
    let mut runner = MockRunner::new();
    for knob in VM_LATENCY_KNOBS {
        runner = runner.with_file(
            knob.runtime_path().to_string_lossy().as_ref(),
            format!("{}\n", knob.target),
        );
    }
    runner
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens")
}

/// Runs apply against a system, returning the recorded changes.
fn apply_against(runner: &MockRunner) -> (Vec<Change>, Result<(), StepError>) {
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, runner),
        VmLatency::id_const(),
        runner,
        &mut log,
    );
    let outcome = VmLatency.apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn probe_on_an_untuned_kernel_says_it_applies() {
    let runner = system_at("999");
    let facts = facts();
    assert_eq!(
        VmLatency
            .probe(&CoreCx::new(&facts, &runner))
            .expect("probes"),
        Probe::Applicable
    );
}

#[test]
fn probe_when_every_parameter_already_holds_its_target_says_so() {
    let runner = system_tuned();
    let facts = facts();
    match VmLatency
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::AlreadyApplied { evidence } => assert!(evidence.contains("all 5"), "{evidence}"),
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn one_parameter_still_wrong_keeps_the_step_applicable() {
    // Four of five already set is not done, and must not report as done.
    let mut runner = MockRunner::new();
    for (index, knob) in VM_LATENCY_KNOBS.into_iter().enumerate() {
        let value = if index == 0 { "999" } else { knob.target };
        runner = runner.with_file(
            knob.runtime_path().to_string_lossy().as_ref(),
            format!("{value}\n"),
        );
    }
    let facts = facts();
    assert_eq!(
        VmLatency
            .probe(&CoreCx::new(&facts, &runner))
            .expect("probes"),
        Probe::Applicable
    );
}

#[test]
fn a_kernel_with_none_of_these_parameters_is_not_applicable() {
    let runner = MockRunner::new();
    let facts = facts();
    match VmLatency
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::NotApplicable { reason } => assert!(reason.contains("none of these"), "{reason}"),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn planning_changes_nothing() {
    let runner = system_at("999");
    let facts = facts();
    let plan = VmLatency
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");

    assert!(runner.commands().is_empty(), "plan ran a command");
    assert_eq!(runner.file(VM_LATENCY_DROPIN), None, "plan wrote a file");
    // One CreateFile plus one SetSysctl per parameter.
    assert_eq!(plan.actions.len(), VM_LATENCY_KNOBS.len() + 1);
    assert!(plan.summary.contains("5 of 5"), "{}", plan.summary);
}

#[test]
fn apply_persists_before_it_sets_any_runtime_value() {
    let runner = system_at("999");
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    assert!(matches!(recorded[0], Change::FileWritten { .. }));
    assert_eq!(recorded.len(), VM_LATENCY_KNOBS.len() + 1);
    assert!(recorded[1..]
        .iter()
        .all(|change| matches!(change, Change::SysctlRuntime { .. })));
}

#[test]
fn each_parameter_records_its_own_previous_value() {
    // One record per key, so rollback restores each independently rather than
    // resetting the group to one shared value.
    let runner = system_at("777");
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let restored: Vec<_> = recorded
        .iter()
        .filter_map(|change| match change {
            Change::SysctlRuntime { key, previous } => Some((key.clone(), previous.clone())),
            Change::FileWritten { .. }
            | Change::FileRemoved { .. }
            | Change::SysfsWrite { .. }
            | Change::PackagesInstalled { .. }
            | Change::SystemdUnit { .. }
            | Change::AptRepository { .. }
            | Change::ScxScheduler { .. }
            | Change::DirCreated { .. }
            | Change::DirTreeInstalled { .. } => None,
        })
        .collect();

    assert_eq!(restored.len(), VM_LATENCY_KNOBS.len());
    assert!(restored.iter().all(|(_, previous)| previous == "777"));
    for knob in VM_LATENCY_KNOBS {
        assert!(
            restored.iter().any(|(key, _)| key == knob.key),
            "{} was not recorded",
            knob.key
        );
    }
}

#[test]
fn the_dropin_names_only_parameters_this_kernel_has() {
    // A drop-in naming an unknown parameter makes `sysctl --system` report an
    // error on every boot.
    let runner = MockRunner::new().with_file("/proc/sys/vm/dirty_ratio", "20\n");
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(VM_LATENCY_DROPIN).expect("drop-in written");
    assert!(written.contains("vm.dirty_ratio = 8"), "{written}");
    assert!(
        !written.contains("compaction_proactiveness"),
        "named a parameter this kernel does not have: {written}"
    );
}

#[test]
fn the_file_it_writes_carries_the_marker_and_explains_each_value() {
    let runner = system_at("999");
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(VM_LATENCY_DROPIN).expect("drop-in written");
    assert!(written.contains(MANAGED_HEADER));
    assert!(written.contains("step=core.sysctl.vm-latency"));
    for knob in VM_LATENCY_KNOBS {
        assert!(
            written.contains(&format!("{} = {}", knob.key, knob.target)),
            "{} missing from {written}",
            knob.key
        );
        assert!(written.contains(knob.why), "{} unexplained", knob.key);
    }
}

#[test]
fn verify_fails_when_the_values_did_not_move() {
    let runner = system_at("999");
    let facts = facts();
    let verification = VmLatency
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn verify_passes_only_when_every_value_and_the_file_are_right() {
    let runner = system_tuned().with_file(VM_LATENCY_DROPIN, "vm.dirty_ratio = 8");
    let facts = facts();
    let verification = VmLatency
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");

    assert!(verification.passed());
    assert_eq!(verification.total_count(), VM_LATENCY_KNOBS.len() + 1);
}

#[test]
fn correct_values_without_the_file_still_fail_verification() {
    // The change would not survive a reboot, so it is not done.
    let runner = system_tuned();
    let facts = facts();
    let verification = VmLatency
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");

    assert!(!verification.passed());
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn apply_then_rollback_restores_every_prior_value() {
    let runner = system_at("777");
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            VmLatency::id_const(),
            &runner,
            &mut log,
        );
        VmLatency.apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        VmLatency::id_const(),
        &runner,
        &mut log,
    );
    VmLatency.rollback(&recorded, &mut cx).expect("rolls back");

    assert_eq!(runner.file(VM_LATENCY_DROPIN), None);
    for knob in VM_LATENCY_KNOBS {
        let restore = format!("sudo sysctl -w {}=777", knob.key);
        assert!(
            runner.commands().contains(&restore),
            "{restore} was never issued: {:?}",
            runner.commands()
        );
    }
}

#[test]
fn apply_failing_midway_still_leaves_an_undoable_record() {
    // Sweeps the failure point across the whole command sequence. At every
    // position, whatever was already done must be described by the records the
    // step recorded, or that part of the change is unrecoverable.
    for failure_point in 0..8 {
        let runner = system_at("999").failing_at(failure_point);
        let (recorded, outcome) = apply_against(&runner);

        if outcome.is_ok() {
            continue;
        }

        let mutating_commands = runner
            .commands()
            .iter()
            .filter(|cmd| cmd.starts_with("sudo"))
            .count();
        assert!(
            recorded.len() >= mutating_commands,
            "failure at {failure_point} ran {mutating_commands} mutating commands \
             but recorded only {} undo records",
            recorded.len()
        );
    }
}
