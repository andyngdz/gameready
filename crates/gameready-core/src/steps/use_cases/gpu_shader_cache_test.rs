use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};

const ENVIRONMENT_D: &str = "/home/tester/.config/environment.d";
const FRAGMENT: &str = "/home/tester/.config/environment.d/99-gameready-shader-cache.conf";

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

fn step() -> ShaderCache {
    ShaderCache::with_environment_d(PathBuf::from(ENVIRONMENT_D))
}

/// A machine whose first card reports the given PCI vendor id.
fn machine_with_card(vendor_id: &str) -> MockRunner {
    MockRunner::new().with_file(
        "/sys/class/drm/card0/device/vendor",
        format!("{vendor_id}\n"),
    )
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
        ShaderCache::id_const(),
        runner,
        &mut log,
    );
    let outcome = step().apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn probe_on_a_machine_with_a_known_card_says_it_applies() {
    let runner = machine_with_card("0x10de");
    let facts = facts();
    assert_eq!(
        step().probe(&CoreCx::new(&facts, &runner)).expect("probes"),
        Probe::Applicable
    );
}

#[test]
fn a_machine_with_no_recognised_card_is_not_applicable() {
    let runner = MockRunner::new();
    let facts = facts();
    match step().probe(&CoreCx::new(&facts, &runner)).expect("probes") {
        Probe::NotApplicable { reason } => assert_eq!(reason, NO_SUPPORTED_GPU),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn probe_when_the_fragment_already_sets_everything_says_so() {
    let runner = machine_with_card("0x1002")
        .with_file(FRAGMENT, "MESA_SHADER_CACHE_MAX_SIZE=12G\n".to_owned());
    let facts = facts();
    match step().probe(&CoreCx::new(&facts, &runner)).expect("probes") {
        Probe::AlreadyApplied { evidence } => assert!(evidence.contains(FRAGMENT), "{evidence}"),
        other @ (Probe::Applicable
        | Probe::NotApplicable { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected already applied, got {other:?}"),
    }
}

#[test]
fn a_fragment_left_by_an_older_version_is_rewritten_not_skipped() {
    // Half the settings present is not applied. Reporting it as done would
    // leave the cache on its 1GB default forever.
    let runner =
        machine_with_card("0x10de").with_file(FRAGMENT, "__GL_SHADER_DISK_CACHE=1\n".to_owned());
    let facts = facts();
    assert_eq!(
        step().probe(&CoreCx::new(&facts, &runner)).expect("probes"),
        Probe::Applicable
    );
}

#[test]
fn planning_changes_nothing() {
    let runner = machine_with_card("0x10de");
    let facts = facts();
    let plan = step().plan(&CoreCx::new(&facts, &runner)).expect("plans");

    assert!(runner.commands().is_empty(), "plan ran a command");
    assert_eq!(runner.file(FRAGMENT), None, "plan wrote a file");
    assert!(plan.summary.contains("NVIDIA"), "{}", plan.summary);
    assert_eq!(plan.actions.len(), 1);
}

#[test]
fn apply_creates_the_directory_before_writing_into_it() {
    let runner = machine_with_card("0x10de");
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    assert!(matches!(recorded[0], Change::DirCreated { .. }));
    assert!(matches!(recorded[1], Change::FileWritten { .. }));
    assert!(runner
        .commands()
        .iter()
        .any(|cmd| cmd.contains("mkdir -p") && cmd.contains(ENVIRONMENT_D)));
}

#[test]
fn apply_does_not_record_a_directory_it_did_not_create() {
    // environment.d is a shared XDG directory. Recording it as ours would make
    // rollback try to take away something gameready never made.
    let runner = machine_with_card("0x10de").with_file(ENVIRONMENT_D, String::new());
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0], Change::FileWritten { .. }));
}

#[test]
fn apply_writes_as_the_user_never_as_root() {
    // A root-owned file in the user's home is one they cannot later edit or
    // delete themselves.
    let runner = machine_with_card("0x1002");
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    match &recorded[1] {
        Change::FileWritten { privilege, .. } => assert_eq!(*privilege, Privilege::User),
        other @ (Change::FileRemoved { .. }
        | Change::SysctlRuntime { .. }
        | Change::SysfsWrite { .. }
        | Change::PackagesInstalled { .. }
        | Change::SystemdUnit { .. }
        | Change::AptRepository { .. }
        | Change::ScxScheduler { .. }
        | Change::DirCreated { .. }
        | Change::DirTreeInstalled { .. }) => panic!("expected a file record, got {other:?}"),
    }
}

#[test]
fn apply_writes_the_settings_for_the_card_that_is_actually_there() {
    let runner = machine_with_card("0x1002");
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(FRAGMENT).expect("fragment written");
    assert!(
        written.contains("MESA_SHADER_CACHE_MAX_SIZE=12G"),
        "{written}"
    );
    assert!(!written.contains("__GL_"), "{written}");
}

#[test]
fn apply_refuses_when_the_card_vanished_between_probe_and_apply() {
    let runner = MockRunner::new();
    let (recorded, outcome) = apply_against(&runner);

    assert!(matches!(outcome, Err(StepError::PreconditionLost { .. })));
    assert!(recorded.is_empty(), "refusal still changed something");
}

#[test]
fn verify_fails_when_nothing_was_written() {
    let runner = machine_with_card("0x10de");
    let facts = facts();
    let verification = step()
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn verify_passes_once_the_fragment_holds_every_setting() {
    let runner = machine_with_card("0x10de");
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let facts = facts();
    let verification = step()
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");

    assert!(verification.passed());
    // The file, plus one check per NVIDIA setting.
    assert_eq!(verification.total_count(), 3);
}

#[test]
fn apply_then_rollback_takes_the_fragment_back_off() {
    let runner = machine_with_card("0x10de");
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            ShaderCache::id_const(),
            &runner,
            &mut log,
        );
        step().apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };
    assert!(runner.file(FRAGMENT).is_some(), "fragment written");

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        ShaderCache::id_const(),
        &runner,
        &mut log,
    );
    step().rollback(&recorded, &mut cx).expect("rolls back");

    assert_eq!(runner.file(FRAGMENT), None);
    // rmdir, never rm -rf: a directory other software has since used stays.
    assert!(runner
        .commands()
        .iter()
        .any(|cmd| cmd.starts_with("rmdir") && cmd.contains(ENVIRONMENT_D)));
    assert!(
        !runner.commands().iter().any(|cmd| cmd.contains("rm -rf")),
        "rollback used a recursive delete: {:?}",
        runner.commands()
    );
}

#[test]
fn apply_failing_midway_still_leaves_an_undoable_record() {
    for failure_point in 0..4 {
        let runner = machine_with_card("0x10de").failing_at(failure_point);
        let (recorded, outcome) = apply_against(&runner);

        if outcome.is_ok() {
            continue;
        }

        let mutating_commands = runner
            .commands()
            .iter()
            .filter(|cmd| cmd.starts_with("mkdir"))
            .count();
        assert!(
            recorded.len() >= mutating_commands,
            "failure at {failure_point} ran {mutating_commands} mutating commands \
             but recorded only {} undo records",
            recorded.len()
        );
    }
}
