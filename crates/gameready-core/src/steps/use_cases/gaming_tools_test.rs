use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::Apt;
use crate::journal::{Journal, RunId, StatePaths};

const TOOL_BINARIES: [&str; 2] = ["gamemoded", "mangohud"];

/// A Debian box where every named package is in the archive but none is
/// installed. `dpkg-query` failing is how apt reports "not installed", and an
/// `apt-cache show` that prints a stanza is how it reports "available".
fn debian_box() -> MockRunner {
    let mut runner = MockRunner::new();
    for package in ["gamemode", "mangohud"] {
        runner = runner
            .failing(format!(
                "dpkg-query --showformat=${{Version}} --show {package}"
            ))
            .answering(
                format!("apt-cache show {package}"),
                format!("Package: {package}\nVersion: 1.0\n"),
            );
    }
    runner
}

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

#[test]
fn probe_reports_applicable_when_tools_are_missing() {
    let runner = debian_box();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert!(matches!(
        GamingTools.probe(&cx).expect("probed"),
        Probe::Applicable
    ));
}

#[test]
fn probe_reports_already_applied_when_every_binary_is_on_path() {
    let mut runner = debian_box();
    for binary in TOOL_BINARIES {
        runner = runner.with_binary(binary);
    }
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let probe = GamingTools.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::AlreadyApplied { .. }), "{probe:?}");
    // The package manager is never consulted once every binary is present, so a
    // user who built gamescope by hand is not told to install it.
    assert!(runner.commands().is_empty());
}

#[test]
fn probe_reports_not_applicable_when_no_missing_tool_is_in_a_repository() {
    // Debian 12 has no gamescope; this is the same shape with all three gone.
    let mut runner = MockRunner::new();
    for package in ["gamemode", "mangohud"] {
        runner = runner
            .failing(format!(
                "dpkg-query --showformat=${{Version}} --show {package}"
            ))
            .failing(format!("apt-cache show {package}"));
    }
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let probe = GamingTools.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::NotApplicable { .. }), "{probe:?}");
}

#[test]
fn probe_reports_unknown_without_package_tooling() {
    // Answering "nothing to install" from a check that never ran would let the
    // summary claim the tools are handled when nothing looked.
    let runner = debian_box();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let probe = GamingTools.probe(&cx).expect("probed");
    assert!(matches!(probe, Probe::Unknown { .. }), "{probe:?}");
}

#[test]
fn a_tool_already_on_path_is_left_out_of_the_plan() {
    let runner = debian_box().with_binary("mangohud");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let plan = GamingTools.plan(&cx).expect("planned");
    assert_eq!(
        plan.actions,
        vec![PlannedAction::InstallPackages {
            names: vec!["gamemode".to_owned()],
        }]
    );
}

#[test]
fn apply_installs_in_one_transaction_and_journals_it() {
    let dir = TempDir::new().expect("temp dir");
    let runner = debian_box();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, GamingTools::id_const(), &runner, &mut journal);

    GamingTools.apply(&mut apply).expect("applied");

    assert_eq!(
        apply.recorded(),
        [Change::PackagesInstalled {
            manager: "apt-get".to_owned(),
            requested: vec!["gamemode".to_owned(), "mangohud".to_owned()],
            newly_installed: vec!["gamemode".to_owned(), "mangohud".to_owned()],
        }]
    );
    assert!(
        runner.commands().iter().any(|command| command
            == "sudo apt-get install --yes --no-install-recommends gamemode mangohud"),
        "expected one transaction, got {:?}",
        runner.commands()
    );
}

#[test]
fn verify_fails_when_a_binary_did_not_appear() {
    let runner = debian_box().with_binary("mangohud");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    let verification = GamingTools.verify(&cx).expect("verified");
    assert_eq!(verification.total_count(), 2);
    assert_eq!(verification.failed_count(), 1);
}

#[test]
fn verify_passes_once_every_binary_is_on_path() {
    let mut runner = debian_box();
    for binary in TOOL_BINARIES {
        runner = runner.with_binary(binary);
    }
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);

    assert_eq!(GamingTools.verify(&cx).expect("verified").failed_count(), 0);
}

#[test]
fn rollback_leaves_installed_packages_alone() {
    // Removing a package is not the inverse of installing one, so the undo runs
    // no command at all and the summary reports what was left behind.
    let dir = TempDir::new().expect("temp dir");
    let runner = debian_box();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, GamingTools::id_const(), &runner, &mut journal);

    let undo = [Change::PackagesInstalled {
        manager: "apt-get".to_owned(),
        requested: vec!["gamemode".to_owned()],
        newly_installed: vec!["gamemode".to_owned()],
    }];
    GamingTools
        .rollback(&undo, &mut apply)
        .expect("rolled back");

    assert!(runner.commands().is_empty());
}

#[test]
fn a_failed_install_leaves_a_journal_that_names_every_package() {
    // The undo record goes down before the transaction runs, so an interrupt
    // partway through still lists everything the transaction could have added.
    let dir = TempDir::new().expect("temp dir");
    let runner = debian_box().failing_at(8);
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner).with_packages(&Apt);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(cx, GamingTools::id_const(), &runner, &mut journal);

    GamingTools
        .apply(&mut apply)
        .expect_err("the install was cut short");

    match apply.recorded() {
        [
            Change::PackagesInstalled {
                newly_installed, ..
            },
        ] => assert_eq!(newly_installed.len(), 2),
        other => panic!("expected one recorded install, got {other:?}"),
    }
}
