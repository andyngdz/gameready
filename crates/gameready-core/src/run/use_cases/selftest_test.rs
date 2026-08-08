use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::journal::{RunId, StatePaths};
use crate::steps::{Conflicts, CpuGovernor, MaxMapCount};

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("journal")
}

#[test]
fn a_step_that_never_applies_is_skipped_rather_than_failed() {
    // core.conflicts only ever reports; it applies on no machine. Reporting it
    // as a broken step would train the reader to ignore the selftest.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);

    let results = selftest(vec![Box::new(Conflicts)], &cx, &runner, &mut journal);

    assert!(matches!(
        results.as_slice(),
        [StepSelftest {
            result: SelftestResult::Skipped { .. },
            ..
        }]
    ));
    assert!(!results[0].is_failure());
}

#[test]
fn a_reporting_step_with_nothing_to_apply_is_skipped() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_binary("systemctl");
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);

    let results = selftest(vec![Box::new(Conflicts)], &cx, &runner, &mut journal);

    assert!(!results[0].is_failure());
}

#[test]
fn a_step_whose_change_does_not_take_effect_fails_at_verify() {
    // The mock never moves the runtime value, so reading it back after apply
    // does not show the target.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new().with_file("/proc/sys/vm/max_map_count", "1048576\n");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);

    let results = selftest(vec![Box::new(MaxMapCount)], &cx, &runner, &mut journal);

    match &results[0].result {
        SelftestResult::Failed { phase, .. } => assert_eq!(*phase, Phase::Verify),
        other @ (SelftestResult::Skipped { .. }
        | SelftestResult::ProbeFailed { .. }
        | SelftestResult::Passed { .. }) => {
            panic!("expected a verify failure, got {other:?}")
        }
    }
}

#[test]
fn a_full_cycle_passes_and_leaves_nothing_behind() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .with_file("/proc/sys/vm/max_map_count", "1048576\n")
        .where_command_writes(
            "sudo sysctl -w vm.max_map_count=2147483642",
            "/proc/sys/vm/max_map_count",
            "2147483642\n",
        )
        .where_command_writes(
            "sudo sysctl -w vm.max_map_count=1048576",
            "/proc/sys/vm/max_map_count",
            "1048576\n",
        );
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);

    let results = selftest(vec![Box::new(MaxMapCount)], &cx, &runner, &mut journal);

    assert_eq!(
        results[0].result,
        SelftestResult::Passed {
            reverted: RevertCheck::Confirmed
        }
    );
    assert!(
        runner.file("/etc/sysctl.d/99-gameready.conf").is_none(),
        "the selftest applied a change and did not take it back"
    );
}

#[test]
fn the_governor_survives_a_full_apply_verify_revert_cycle() {
    // The privileged path with no sudo anywhere: MockRunner::write_sysfs lands
    // the value in the fake file, so apply, verify, revert, and the final check
    // that the change is gone all run against it.
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new()
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor",
            "powersave\n",
        )
        .with_file(
            "/sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors",
            "performance powersave\n",
        );
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);

    let results = selftest(vec![Box::new(CpuGovernor)], &cx, &runner, &mut journal);

    assert_eq!(
        results[0].result,
        SelftestResult::Passed {
            reverted: RevertCheck::Confirmed
        }
    );
    assert_eq!(
        runner
            .file("/sys/devices/system/cpu/cpufreq/policy0/scaling_governor")
            .as_deref(),
        Some("powersave"),
        "the selftest applied a change and did not take it back"
    );
}

#[test]
fn every_step_gets_a_result() {
    let dir = TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let facts = SystemFacts::fixture(Family::Fedora);
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = journal(&dir);

    let results = selftest(
        vec![Box::new(CpuGovernor), Box::new(Conflicts)],
        &cx,
        &runner,
        &mut journal,
    );

    assert_eq!(results.len(), 2);
}
