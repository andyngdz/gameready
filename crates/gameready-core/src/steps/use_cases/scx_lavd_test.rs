use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::Apt;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steps::constants::SCXCTL_BIN;

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

/// A kernel with sched_ext built in and nothing attached, which is the state
/// this project's own machine is in.
fn idle_kernel() -> MockRunner {
    MockRunner::new().with_file(SCHED_EXT_STATE, "disabled\n")
}

/// The same kernel with scx already installed, so no package work is needed.
///
/// The package query is seeded as well as the binary: `scxctl` is on `PATH`
/// precisely because the package is installed, and a fixture where one is true
/// and the other is not describes no real machine.
fn ready_kernel() -> MockRunner {
    idle_kernel()
        .with_binary(SCXCTL_BIN)
        .answering("dpkg-query --showformat=${Version} --show scx", "1.0.19")
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open")
}

#[test]
fn a_kernel_without_sched_ext_can_never_run_this() {
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    match ScxLavd.probe(&cx).expect("probed") {
        Probe::NotApplicable { reason } => assert!(reason.contains("no sched_ext"), "{reason}"),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn an_idle_kernel_with_the_tooling_installed_is_ready_to_go() {
    let runner = ready_kernel();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert!(matches!(
        ScxLavd.probe(&cx).expect("probed"),
        Probe::Applicable
    ));
}

#[test]
fn a_kernel_already_running_lavd_is_left_alone() {
    let runner = MockRunner::new()
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(crate::steps::constants::SCHED_EXT_OPS, "lavd\n")
        .with_binary(SCXCTL_BIN);
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert!(matches!(
        ScxLavd.probe(&cx).expect("probed"),
        Probe::AlreadyApplied { .. }
    ));
}

#[test]
fn a_scheduler_somebody_else_loaded_is_a_conflict_this_run_can_clear() {
    // scxctl is present, so the loader can hand the machine back to the kernel
    // and take the seat; the conflict says so, which is what turns it into a
    // takeover question the user gets to answer.
    let runner = MockRunner::new()
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(crate::steps::constants::SCHED_EXT_OPS, "bpfland\n")
        .with_binary(SCXCTL_BIN);
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    match ScxLavd.probe(&cx).expect("probed") {
        Probe::Conflict { with, yours, .. } => {
            assert_eq!(with, "bpfland");
            let command = yours.expect("scxctl can clear the conflict");
            assert!(command.contains("scxctl stop"), "{command}");
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::NotApplicable { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn a_scheduler_with_no_owner_gameready_can_stop_gets_no_takeover_offer() {
    // No scxctl and no active scx.service: whatever loaded bpfland is not
    // something this run can put back, so the conflict carries no path and the
    // run stands down instead of asking for a takeover it cannot undo.
    let runner = MockRunner::new()
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(crate::steps::constants::SCHED_EXT_OPS, "bpfland\n");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    match ScxLavd.probe(&cx).expect("probed") {
        Probe::Conflict { yours: None, .. } => {}
        other => panic!("expected an unoffered conflict, got {other:?}"),
    }
}

#[test]
fn a_scheduler_run_by_the_scx_unit_is_offerable_because_the_unit_is_the_owner() {
    // Ubuntu's scx.service is running, which is proof of where the scheduler
    // came from; gameready manages that unit, so it can stop it and put the
    // machine back on the user's own config afterwards.
    let runner = MockRunner::new()
        .with_binary("systemctl")
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(crate::steps::constants::SCHED_EXT_OPS, "cosmos\n")
        .answering("systemctl is-enabled scx", "enabled")
        .answering("systemctl is-active scx", "active");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    match ScxLavd.probe(&cx).expect("probed") {
        Probe::Conflict { with, yours, .. } => {
            assert_eq!(with, "cosmos");
            let command = yours.expect("the unit's owner is stoppable");
            assert!(command.contains("systemctl stop scx"), "{command}");
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn a_system_whose_repositories_have_no_scx_is_not_sent_away_to_run_gameready_twice() {
    let runner = idle_kernel()
        .failing("dpkg-query --showformat=${Version} --show scx")
        .failing("apt-cache show scx");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    match ScxLavd.probe(&cx).expect("probed") {
        Probe::NotApplicable { reason } => {
            // Ubuntu is told about the step in this same run that fixes it.
            // The promise of a second run went with the re-probe that made it
            // untrue: this step is held open and looked at again in this pass.
            assert!(reason.contains("core.repo.scx-ppa"), "{reason}");
            assert!(
                !reason.contains("the next time you run gameready"),
                "{reason}"
            );
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn the_plan_names_the_command_it_will_run() {
    let runner = ready_kernel();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let plan = ScxLavd.plan(&cx).expect("planned");
    match &plan.actions[..] {
        [PlannedAction::RunCommand { display }] => {
            assert!(
                display.contains("scxctl start -s lavd -m gaming"),
                "{display}"
            );
        }
        other => panic!("expected one command, got {other:?}"),
    }
}

#[test]
fn apply_loads_the_scheduler_and_records_what_it_replaced() {
    let dir = TempDir::new().expect("temp dir");
    let runner = ready_kernel();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    ScxLavd.apply(&mut apply).expect("applied");

    // The kernel was on its own scheduler, so the undo is a stop rather than a
    // switch back to something.
    assert_eq!(apply.recorded(), [Change::ScxScheduler { previous: None }]);
    assert!(
        runner
            .commands()
            .iter()
            .any(|command| command.contains("scxctl start -s lavd -m gaming")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn rollback_hands_the_cpu_back_without_a_reboot() {
    let dir = TempDir::new().expect("temp dir");
    let runner = ready_kernel();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal = journal(&dir);
    let mut apply = ApplyCx::new(cx, ScxLavd::id_const(), &runner, &mut journal);

    let undo = [Change::ScxScheduler { previous: None }];
    ScxLavd.rollback(&undo, &mut apply).expect("rolled back");

    assert!(
        runner
            .commands()
            .iter()
            .any(|command| command.contains("scxctl stop")),
        "{:?}",
        runner.commands()
    );
}

#[test]
fn verify_fails_when_the_kernel_is_not_actually_running_lavd() {
    let runner = ready_kernel();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let verification = ScxLavd.verify(&cx).expect("verified");
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn verify_passes_once_the_kernel_reports_lavd() {
    let runner = MockRunner::new()
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(crate::steps::constants::SCHED_EXT_OPS, "lavd\n");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert_eq!(ScxLavd.verify(&cx).expect("verified").failed_count(), 0);
}
