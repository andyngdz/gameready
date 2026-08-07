use indoc::indoc;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::{Apt, Pacman};

/// A Debian box where the PPA is configured, so `scx` resolves.
fn debian_with_ppa() -> MockRunner {
    MockRunner::new()
        .failing("dpkg-query --showformat=${Version} --show scx")
        .answering(
            "apt-cache show scx",
            indoc! {"
                Package: scx
                Version: 1.1.1-1
            "},
        )
}

/// An Arch box where both packages are in `extra` and neither is installed.
fn arch_box() -> MockRunner {
    let mut runner = MockRunner::new();
    for package in ["scx-scheds", "scx-tools"] {
        runner = runner.failing(format!("pacman -Q {package}")).answering(
            format!("pacman -Si {package}"),
            format!("Name : {package}\n"),
        );
    }
    runner
}

#[test]
fn ubuntu_needs_one_package_because_the_ppa_does_not_split_them() {
    // The PPA publishes a single binary package named `scx`. Asking for
    // `scx-tools` there would report the step unavailable on a machine that can
    // in fact run it.
    let runner = debian_with_ppa();
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let survey = ScxPackages::read(&cx, &Apt).expect("survey reads");

    assert!(survey.can_install());
    let planned = survey.to_install();
    assert_eq!(planned.len(), 1);
    assert_eq!(planned[0].name, "scx");
}

#[test]
fn arch_needs_both_packages_because_the_loader_ships_separately() {
    // scx-scheds alone installs no scxctl and no loader service, so a step that
    // asked for it on its own would install 179 MB and still not be able to
    // load anything.
    let runner = arch_box();
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let survey = ScxPackages::read(&cx, &Pacman).expect("survey reads");

    let names: Vec<String> = survey
        .to_install()
        .into_iter()
        .map(|package| package.name)
        .collect();
    assert_eq!(names, vec!["scx-scheds", "scx-tools"]);
}

#[test]
fn every_planned_package_carries_what_it_is_and_why_the_run_wants_it() {
    let runner = arch_box();
    let facts = SystemFacts::fixture(Family::Arch);
    let cx = CoreCx::new(&facts, &runner);
    let survey = ScxPackages::read(&cx, &Pacman).expect("survey reads");

    for package in survey.to_install() {
        assert!(!package.what.is_empty(), "{package:?}");
        assert!(!package.why.is_empty(), "{package:?}");
        assert!(package.approx_bytes > 0, "{package:?}");
    }
}

#[test]
fn a_repository_without_scx_cannot_install_it() {
    let runner = MockRunner::new()
        .failing("dpkg-query --showformat=${Version} --show scx")
        .failing("apt-cache show scx");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let survey = ScxPackages::read(&cx, &Apt).expect("survey reads");

    assert!(!survey.can_install());
}

#[test]
fn a_package_already_installed_is_listed_rather_than_fetched_again() {
    let runner =
        MockRunner::new().answering("dpkg-query --showformat=${Version} --show scx", "1.1.1-1\n");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, &runner);
    let survey = ScxPackages::read(&cx, &Apt).expect("survey reads");

    assert!(survey.can_install());
    assert!(survey.to_install().is_empty());
    assert_eq!(survey.already_here(), vec!["scx".to_owned()]);
}
