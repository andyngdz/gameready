use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Fedora)
}

#[test]
fn it_declines_and_names_the_current_governor() {
    let runner = MockRunner::new().with_file(SCALING_GOVERNOR, "schedutil\n");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    match CpuGovernor.probe(&cx).expect("probed") {
        Probe::NotApplicable { reason } => {
            assert!(reason.contains("schedutil"), "{reason}");
            assert!(reason.contains("gamemode"), "{reason}");
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::Unknown { .. }) => panic!("expected NotApplicable, got {other:?}"),
    }
}

#[test]
fn it_still_declines_on_a_machine_with_no_cpufreq() {
    // Normal in a virtual machine. There is nothing to report, and that is not
    // a failure.
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let probe = CpuGovernor.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn it_declines_even_when_the_governor_is_already_performance() {
    // Whatever it is set to, gameready is not the one who set it.
    let runner = MockRunner::new().with_file(SCALING_GOVERNOR, "performance\n");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let probe = CpuGovernor.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn probing_changes_nothing() {
    let runner = MockRunner::new().with_file(SCALING_GOVERNOR, "powersave\n");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    CpuGovernor.probe(&cx).expect("probed");

    assert!(runner.commands().is_empty());
    assert_eq!(
        runner.file(SCALING_GOVERNOR).as_deref(),
        Some("powersave\n")
    );
}

#[test]
fn the_unreachable_apply_refuses_rather_than_doing_nothing() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let mut journal = crate::journal::Journal::open(
        crate::journal::StatePaths::new(dir.path().to_path_buf()),
        crate::journal::RunId::generate(),
    )
    .expect("open");
    let mut apply = ApplyCx::new(cx, CpuGovernor::id_const(), &runner, &mut journal);

    assert!(CpuGovernor.apply(&mut apply).is_err());
    assert!(apply.recorded().is_empty());
}

#[test]
fn it_verifies_nothing_because_it_changed_nothing() {
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    assert_eq!(
        CpuGovernor.verify(&cx).expect("verified").total_count(),
        0,
        "a passing check here would be inventing evidence"
    );
}
