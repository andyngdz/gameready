use tempfile::TempDir;

use super::*;
use crate::facts::SystemFacts;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};

const CONFIG: &str = "/home/tester/.config/gamemode.ini";

fn facts() -> SystemFacts {
    SystemFacts::fixture(crate::facts::Family::Debian)
}

fn step() -> GamemodeConfig {
    GamemodeConfig::with_config(PathBuf::from(CONFIG))
}

/// gamemode installed, and the session carrying the group it needs.
fn ready_machine() -> MockRunner {
    MockRunner::new()
        .with_binary("gamemoded")
        .answering("id -nG", "tester sudo gamemode\n")
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
        GamemodeConfig::id_const(),
        runner,
        &mut log,
    );
    let outcome = step().apply(&mut cx);
    (cx.recorded().to_vec(), outcome)
}

#[test]
fn probe_on_a_ready_machine_says_it_applies() {
    let runner = ready_machine();
    let facts = facts();
    assert_eq!(
        step().probe(&CoreCx::new(&facts, &runner)).expect("probes"),
        Probe::Applicable
    );
}

#[test]
fn without_gamemode_there_is_nothing_to_configure() {
    let runner = MockRunner::new();
    let facts = facts();
    match step().probe(&CoreCx::new(&facts, &runner)).expect("probes") {
        Probe::NotApplicable { reason } => assert!(reason.contains("not installed"), "{reason}"),
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn it_is_reprobed_after_the_step_that_installs_gamemode() {
    // Without this, a machine that gets gamemode in the same run would have to
    // be told to run gameready twice.
    assert_eq!(step().requires(), &[GamingTools::id_const()]);
}

#[test]
fn a_user_outside_the_gamemode_group_gets_the_command_not_a_useless_file() {
    // gamemoded reads the client's own groups, so renice would silently do
    // nothing. Writing the file anyway would look like the step worked.
    let runner = MockRunner::new()
        .with_binary("gamemoded")
        .answering("id -nG", "tester sudo video\n");
    let facts = facts();
    match step().probe(&CoreCx::new(&facts, &runner)).expect("probes") {
        Probe::NotApplicable { reason } => {
            assert!(reason.contains("gamemode group"), "{reason}");
            assert!(reason.contains("usermod"), "{reason}");
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::Conflict { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected not applicable, got {other:?}"),
    }
}

#[test]
fn a_config_the_user_wrote_themselves_is_a_conflict_never_an_overwrite() {
    let theirs = indoc::indoc! {"
        [general]
        renice=5
        "};
    let runner = ready_machine().with_file(CONFIG, theirs.to_owned());
    let facts = facts();
    match step().probe(&CoreCx::new(&facts, &runner)).expect("probes") {
        Probe::Conflict { with, detail, .. } => {
            assert!(with.contains("your own"), "{with}");
            assert!(detail.contains(CONFIG), "{detail}");
        }
        other @ (Probe::Applicable
        | Probe::AlreadyApplied { .. }
        | Probe::NotApplicable { .. }
        | Probe::UpdateAvailable { .. }
        | Probe::Unknown { .. }) => panic!("expected a conflict, got {other:?}"),
    }
}

#[test]
fn a_config_gameready_already_wrote_is_already_applied() {
    let runner = ready_machine();
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let facts = facts();
    assert!(matches!(
        step().probe(&CoreCx::new(&facts, &runner)).expect("probes"),
        Probe::AlreadyApplied { .. }
    ));
}

#[test]
fn planning_changes_nothing() {
    let runner = ready_machine();
    let facts = facts();
    let plan = step().plan(&CoreCx::new(&facts, &runner)).expect("plans");

    assert!(runner.commands().is_empty(), "plan ran a command");
    assert_eq!(runner.file(CONFIG), None, "plan wrote a file");
    assert_eq!(plan.actions.len(), 1);
    assert!(plan.summary.contains("20"), "{}", plan.summary);
}

#[test]
fn apply_writes_the_one_setting_that_is_not_already_a_default() {
    let runner = ready_machine();
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(CONFIG).expect("config written");
    assert!(written.contains("[general]"), "{written}");
    assert!(written.contains("renice=20"), "{written}");
    // Every one of these is already gamemode's own default. Writing them would
    // restate the defaults and then own them forever.
    for already_default in ["ioprio", "inhibit_screensaver", "disable_splitlock"] {
        assert!(
            !written.contains(already_default),
            "{already_default} is already the default: {written}"
        );
    }
}

#[test]
fn the_file_it_writes_carries_the_marker_doctor_looks_for() {
    let runner = ready_machine();
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let written = runner.file(CONFIG).expect("config written");
    assert!(written.contains(MANAGED_HEADER), "{written}");
    assert!(written.contains("step=core.gamemode.config"), "{written}");
}

#[test]
fn apply_writes_as_the_user_never_as_root() {
    let runner = ready_machine();
    let (recorded, outcome) = apply_against(&runner);
    outcome.expect("applies");

    match &recorded[0] {
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
fn verify_fails_when_nothing_was_written() {
    let runner = ready_machine();
    let facts = facts();
    let verification = step()
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(!verification.passed());
}

#[test]
fn verify_passes_once_the_setting_is_in_the_file() {
    let runner = ready_machine();
    let (_, outcome) = apply_against(&runner);
    outcome.expect("applies");

    let facts = facts();
    let verification = step()
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");
    assert!(verification.passed());
    assert_eq!(verification.total_count(), 2);
}

#[test]
fn an_empty_file_fails_verification_rather_than_counting_as_written() {
    let runner = ready_machine().with_file(CONFIG, "[general]\n".to_owned());
    let facts = facts();
    let verification = step()
        .verify(&CoreCx::new(&facts, &runner))
        .expect("verifies");

    assert!(!verification.passed());
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn apply_then_rollback_takes_the_file_back_off() {
    let runner = ready_machine();
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let facts = facts();

    let recorded = {
        let mut cx = ApplyCx::new(
            CoreCx::new(&facts, &runner),
            GamemodeConfig::id_const(),
            &runner,
            &mut log,
        );
        step().apply(&mut cx).expect("applies");
        cx.recorded().to_vec()
    };
    assert!(runner.file(CONFIG).is_some(), "config written");

    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        GamemodeConfig::id_const(),
        &runner,
        &mut log,
    );
    step().rollback(&recorded, &mut cx).expect("rolls back");

    assert_eq!(runner.file(CONFIG), None);
}
