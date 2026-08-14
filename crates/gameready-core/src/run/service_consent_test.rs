//! The install-consent branches of `apply_plan`.
//!
//! Split from `service_test.rs` because they need package tooling standing up,
//! which the rest of the executor's tests deliberately run without.

use tempfile::TempDir;

use super::service_test::{facts, Fake};
use super::*;
use crate::facts::PackageManagerKind;
use crate::improvement::{Dependency, DependencyKind, PackageSpec, Probe, SkipReason};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::Apt;
use crate::journal::{RunId, StatePaths};

/// Runs with package tooling wired in, so the consent branches are reachable.
fn run_with_packages(
    steps: Vec<Box<dyn CoreImprovement>>,
    runner: &MockRunner,
    consent: InstallConsent,
) -> RunReport {
    let dir = TempDir::new().expect("temp dir");
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");
    let system = facts();
    let packages = Apt;
    let cx = CoreCx::new(&system, runner).with_packages(&packages);
    execute(
        steps,
        &cx,
        &mut journal,
        Mode::Apply,
        consent,
        &[],
        &mut |_| {},
    )
    .expect("run completes")
}

/// A step declaring one missing package, so the install screen has something to
/// ask about.
fn needs_mangohud(id: &'static str) -> Box<dyn CoreImprovement> {
    Box::new(Fake {
        id,
        probe_result: Probe::Applicable,
        applies: true,
        verifies: true,
        deps: vec![Dependency::new(
            DependencyKind::Package {
                spec: PackageSpec::uniform("mangohud", 5_000_000),
            },
            "an in-game frame-rate overlay",
            "so you can measure whether any of this helped",
        )],
        self_installs: Vec::new(),
    })
}

/// Answers the apt queries that make mangohud look installable.
fn apt_offering_mangohud() -> MockRunner {
    MockRunner::new().answering("apt-cache show mangohud", "Package: mangohud\nVersion: 0.8")
}

#[test]
fn planning_installs_nothing_and_writes_nothing() {
    let runner = apt_offering_mangohud();
    let system = facts();
    let packages = Apt;
    let cx = CoreCx::new(&system, &runner).with_packages(&packages);

    let plan = plan_run(vec![needs_mangohud("test.a")], &cx, &mut |_| {});

    assert!(
        plan.installs_anything(),
        "mangohud should read as installable"
    );
    assert!(runner.paths().is_empty(), "planning wrote a file");
    assert!(
        !runner
            .commands()
            .iter()
            .any(|cmd| cmd.contains("apt-get install")),
        "planning installed something: {:?}",
        runner.commands()
    );
}

#[test]
fn declining_the_install_skips_only_the_steps_that_needed_it() {
    let runner = apt_offering_mangohud();
    let report = run_with_packages(
        vec![
            needs_mangohud("test.a"),
            Box::new(Fake::applicable("test.b")),
        ],
        &runner,
        InstallConsent::Declined,
    );

    assert!(
        !runner
            .commands()
            .iter()
            .any(|cmd| cmd.contains("apt-get install")),
        "declined but installed anyway: {:?}",
        runner.commands()
    );
    assert_eq!(report.applied(), 1, "the step needing nothing should run");
    assert!(matches!(
        report
            .steps
            .iter()
            .find(|s| s.step.as_str() == "test.a")
            .map(|s| &s.outcome),
        Some(Outcome::Skipped {
            reason: SkipReason::UserDeclined
        })
    ));
}

#[test]
fn granting_the_install_runs_one_package_transaction() {
    let runner = apt_offering_mangohud();
    let report = run_with_packages(
        vec![needs_mangohud("test.a")],
        &runner,
        InstallConsent::Granted,
    );

    assert!(
        runner.commands().iter().any(|cmd| cmd.contains("mangohud")),
        "granted but never installed: {:?}",
        runner.commands()
    );
    assert_eq!(report.applied(), 1);
}

/// A step that installs a package inside its own `apply`, the way
/// `core.pkg.tools` does.
fn installs_gamemode(id: &'static str) -> Box<dyn CoreImprovement> {
    Box::new(Fake {
        id,
        probe_result: Probe::Applicable,
        applies: true,
        verifies: true,
        deps: Vec::new(),
        self_installs: vec!["gamemode".to_owned()],
    })
}

#[test]
fn a_step_that_installs_its_own_packages_is_counted_before_the_question() {
    // core.pkg.tools does not declare a dependency, it installs in apply. A
    // plan that missed that would let the run install gamemode after a screen
    // that asked about nothing.
    let runner = MockRunner::new();
    let system = facts();
    let packages = Apt;
    let cx = CoreCx::new(&system, &runner).with_packages(&packages);

    let plan = plan_run(vec![installs_gamemode("test.a")], &cx, &mut |_| {});

    assert!(plan.installs_anything());
    assert_eq!(plan.installs(PackageManagerKind::Apt).len(), 1);
    assert_eq!(
        plan.installs(PackageManagerKind::Apt)[0].package,
        "gamemode"
    );
}

#[test]
fn declining_skips_a_step_that_would_have_installed_its_own_packages() {
    let runner = MockRunner::new();
    let report = run_with_packages(
        vec![
            installs_gamemode("test.a"),
            Box::new(Fake::applicable("test.b")),
        ],
        &runner,
        InstallConsent::Declined,
    );

    assert_eq!(report.applied(), 1, "the step needing nothing should run");
    assert!(matches!(
        report
            .steps
            .iter()
            .find(|s| s.step.as_str() == "test.a")
            .map(|s| &s.outcome),
        Some(Outcome::Skipped {
            reason: SkipReason::UserDeclined
        })
    ));
}
