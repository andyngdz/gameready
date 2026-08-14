use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::MANAGED_HEADER;

const NVME_SCHEDULER: &str = "/sys/block/nvme0n1/queue/scheduler";
const SDB_SCHEDULER: &str = "/sys/block/sdb/queue/scheduler";

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

/// A fake `/sys/block` holding the given `(name, rotational, scheduler)` disks.
fn mock_with(devices: &[(&str, &str, &str)]) -> MockRunner {
    let mut runner = MockRunner::new();
    for (name, rotational, scheduler) in devices {
        runner = runner
            .with_file(
                format!("/sys/block/{name}/queue/rotational"),
                format!("{rotational}\n"),
            )
            .with_file(
                format!("/sys/block/{name}/queue/scheduler"),
                format!("{scheduler}\n"),
            );
    }
    runner
}

/// Two disks that both want a different scheduler than they have.
fn two_changing() -> MockRunner {
    mock_with(&[
        ("nvme0n1", "0", "[mq-deadline] none"),
        ("sdb", "0", "[none] mq-deadline"),
    ])
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens")
}

fn apply_against(runner: &MockRunner) -> (Vec<Change>, Result<(), StepError>) {
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, runner),
        IoScheduler::id_const(),
        runner,
        &mut log,
    );
    let outcome = IoScheduler.apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn probe_when_a_disk_is_on_the_wrong_scheduler_says_it_applies() {
    let runner = mock_with(&[("nvme0n1", "0", "[mq-deadline] none")]);
    let facts = facts();
    let probe = IoScheduler
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes");
    assert_eq!(probe, Probe::Applicable);
}

#[test]
fn probe_when_every_disk_is_already_right_says_so() {
    let runner = mock_with(&[("nvme0n1", "0", "[none] mq-deadline")]);
    let facts = facts();
    match IoScheduler
        .probe(&CoreCx::new(&facts, &runner))
        .expect("probes")
    {
        Probe::AlreadyApplied { evidence } => assert!(evidence.contains("nvme0n1 on none")),
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn probe_with_no_tunable_disks_is_not_applicable() {
    let runner = MockRunner::new();
    let facts = facts();
    assert!(matches!(
        IoScheduler
            .probe(&CoreCx::new(&facts, &runner))
            .expect("probes"),
        Probe::NotApplicable { .. }
    ));
}

#[test]
fn the_plan_writes_a_rule_and_one_action_per_changing_disk() {
    let runner = mock_with(&[("nvme0n1", "0", "[mq-deadline] none")]);
    let facts = facts();
    let plan = IoScheduler
        .plan(&CoreCx::new(&facts, &runner))
        .expect("plans");
    assert!(plan.summary.contains("nvme0n1 -> none"));
    // One CreateFile for the rule, one WriteSysfs for the disk.
    assert_eq!(plan.actions.len(), 2);
}

#[test]
fn apply_persists_the_rule_before_it_writes_any_scheduler() {
    let runner = two_changing();
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    assert!(matches!(recorded[0], Change::FileWritten { .. }));
    assert!(matches!(recorded[1], Change::SysfsWrite { .. }));
}

#[test]
fn apply_records_each_disk_previous_scheduler_for_rollback() {
    let runner = two_changing();
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let previous: Vec<&str> = recorded
        .iter()
        .filter_map(|change| match change {
            Change::SysfsWrite { previous, .. } => Some(previous.as_str()),
            Change::FileWritten { .. }
            | Change::SteamConfigWritten { .. }
            | Change::FileRemoved { .. }
            | Change::SysctlRuntime { .. }
            | Change::PackagesInstalled { .. }
            | Change::SystemdUnit { .. }
            | Change::DirCreated { .. }
            | Change::DirTreeInstalled { .. } => None,
        })
        .collect();
    assert_eq!(previous, ["mq-deadline", "none"]);
}

#[test]
fn the_rule_it_writes_carries_the_marker_doctor_looks_for() {
    let runner = two_changing();
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(IO_SCHEDULER_RULE).expect("rule written");
    assert!(written.contains(MANAGED_HEADER));
    assert!(written.contains("KERNEL==\"nvme[0-9]*\""));
}

#[test]
fn verify_passes_when_every_disk_is_on_target_and_the_rule_exists() {
    let runner = mock_with(&[
        ("nvme0n1", "0", "[none] mq-deadline"),
        ("sdb", "0", "[mq-deadline] none"),
    ])
    .with_file(IO_SCHEDULER_RULE, "rule");
    let facts = facts();
    let verification = IoScheduler
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(verification.passed());
}

#[test]
fn verify_fails_when_a_disk_is_not_on_its_target() {
    let runner =
        mock_with(&[("nvme0n1", "0", "[mq-deadline] none")]).with_file(IO_SCHEDULER_RULE, "rule");
    let facts = facts();
    let verification = IoScheduler
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn apply_then_rollback_restores_every_scheduler_and_removes_the_rule() {
    let runner = two_changing();
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            IoScheduler::id_const(),
            &runner,
            &mut log,
        );
        IoScheduler.apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };

    assert!(runner.file(IO_SCHEDULER_RULE).is_some(), "rule was written");

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        IoScheduler::id_const(),
        &runner,
        &mut log,
    );
    IoScheduler
        .rollback(&recorded, &mut cx)
        .expect("rolls back");

    assert_eq!(runner.file(IO_SCHEDULER_RULE), None, "rule removed");
    assert_eq!(runner.file(NVME_SCHEDULER).as_deref(), Some("mq-deadline"));
    assert_eq!(runner.file(SDB_SCHEDULER).as_deref(), Some("none"));
}

#[test]
fn apply_failing_midway_still_leaves_an_undoable_record() {
    for failure_point in 0..3 {
        let runner = two_changing().failing_at(failure_point);
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
