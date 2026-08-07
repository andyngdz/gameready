use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;
use crate::infra::pkg::Apt;

/// A Debian box where every named package is in the archive but none is
/// installed.
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
fn a_tool_already_on_path_is_reported_as_present_not_as_a_candidate() {
    let runner = debian_box().with_binary("gamemoded");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    assert_eq!(present(&cx), vec!["gamemode".to_owned()]);
    assert_eq!(
        absent(&cx)
            .iter()
            .map(|tool| tool.binary)
            .collect::<Vec<_>>(),
        vec!["mangohud"]
    );
}

#[test]
fn every_planned_package_carries_what_it_is_and_why_the_run_wants_it() {
    // These two sentences are the whole basis on which a user says yes, so an
    // empty one would put an unanswerable question on the screen.
    let runner = debian_box();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let survey = ToolSurvey::read(&cx, &Apt).expect("survey reads");

    let planned = survey.planned();
    assert_eq!(planned.len(), 2, "{planned:?}");
    for package in &planned {
        assert!(!package.what.is_empty(), "{package:?}");
        assert!(!package.why.is_empty(), "{package:?}");
        assert!(package.approx_bytes > 0, "{package:?}");
    }
}

#[test]
fn planned_and_installable_name_the_same_packages() {
    // The screen asks about `planned`, and `apply` fetches `installable`. A
    // disagreement between them installs something nobody agreed to.
    let runner = debian_box();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let survey = ToolSurvey::read(&cx, &Apt).expect("survey reads");

    let planned: Vec<String> = survey
        .planned()
        .into_iter()
        .map(|package| package.name)
        .collect();
    assert_eq!(planned, survey.installable());
}
