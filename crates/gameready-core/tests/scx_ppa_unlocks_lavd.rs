//! The case the whole re-probe feature exists for.
//!
//! On Ubuntu, scx lives in a PPA that gameready adds itself. Probing every step
//! before applying any of them meant the scheduler step read "not in your
//! repositories" a second before the repository was added, and told the user to
//! come back and run gameready again. Both real steps run here, against a fake
//! apt that starts answering differently once the PPA lands.

// An integration test is its own crate, so the crate-level allow in lib.rs does
// not reach here. A test reports failure by panicking either way.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use gameready_core::facts::{Family, SystemFacts};
use gameready_core::improvement::CoreCx;
use gameready_core::improvement::{CoreImprovement, OutcomeKind};
use gameready_core::infra::exec::MockRunner;
use gameready_core::infra::pkg::Apt;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{execute, InstallConsent, Mode, RunReport};
use gameready_core::steps::{ScxLavd, ScxPpa};
use tempfile::TempDir;

const SCHED_EXT_STATE: &str = "/sys/kernel/sched_ext/state";
const SCHED_EXT_OPS: &str = "/sys/kernel/sched_ext/root/ops";
const INSTALL_SCX: &str = "sudo apt-get install --yes --no-install-recommends scx";
const START_SCX: &str = "sudo systemctl enable --now scx";
const ADD_PPA: &str = "sudo add-apt-repository --yes ppa:arighi/sched-ext";
const APT_SEES_SCX: &str = "apt-cache show scx";
/// What `apt-cache show scx` prints once the PPA is configured.
fn scx_in_the_ppa() -> &'static str {
    indoc::indoc! {"
        Package: scx
        Version: 1.0.19
        "}
}

/// Ubuntu before the PPA: a kernel that can run sched_ext, and an apt that has
/// never heard of scx until `add-apt-repository` runs.
fn ubuntu_without_the_ppa() -> MockRunner {
    MockRunner::new()
        .with_file(SCHED_EXT_STATE, "disabled\n")
        .failing("dpkg-query --showformat=${Version} --show scx")
        .failing(APT_SEES_SCX)
        .where_command_changes_answer(ADD_PPA, APT_SEES_SCX, scx_in_the_ppa())
        // Installing it is what puts the loader on the machine, and starting
        // the unit is what attaches the scheduler. Modelling both is what lets
        // the scheduler step verify itself rather than only appear to run.
        .where_command_changes_answer(
            INSTALL_SCX,
            "dpkg-query --showformat=${Version} --show scx",
            "1.0.19",
        )
        .where_command_writes(START_SCX, SCHED_EXT_STATE, "enabled\n")
        .where_command_writes(START_SCX, SCHED_EXT_OPS, "lavd\n")
}

fn run(runner: &MockRunner, consent: InstallConsent) -> RunReport {
    let dir = TempDir::new().expect("temp dir");
    let facts = SystemFacts::fixture(Family::Debian);
    let packages = Apt;
    let cx = CoreCx::new(&facts, runner).with_packages(&packages);
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let steps: Vec<Box<dyn CoreImprovement>> = vec![Box::new(ScxPpa), Box::new(ScxLavd)];

    execute(steps, &cx, &mut journal, Mode::Apply, consent, &mut |_| {}).expect("the run completes")
}

fn outcome_of(report: &RunReport, id: &str) -> OutcomeKind {
    report
        .steps
        .iter()
        .find(|step| step.step.as_str() == id)
        .unwrap_or_else(|| panic!("{id} is missing from the report"))
        .outcome
        .kind()
}

#[test]
fn adding_the_ppa_lets_the_scheduler_step_run_in_the_same_pass() {
    let runner = ubuntu_without_the_ppa();
    let report = run(&runner, InstallConsent::Granted);

    assert_eq!(
        outcome_of(&report, "core.repo.scx-ppa"),
        OutcomeKind::Applied
    );
    assert_eq!(
        outcome_of(&report, "core.sched.scx-lavd"),
        OutcomeKind::Applied,
        "the scheduler waited for the next run instead of using this one"
    );
}

#[test]
fn nothing_in_the_report_tells_the_user_to_run_gameready_again() {
    let runner = ubuntu_without_the_ppa();
    let report = run(&runner, InstallConsent::Granted);

    for step in &report.steps {
        let detail = step.outcome.detail().unwrap_or_default();
        assert!(
            !detail.contains("next time you run gameready"),
            "{}: {detail}",
            step.step
        );
    }
}

#[test]
fn declining_the_install_skips_the_scheduler_rather_than_fetching_it_anyway() {
    // The consent guarantee. The held-open step's packages went onto the same
    // screen as everybody else's, so a no there is a no for it too.
    let runner = ubuntu_without_the_ppa();
    let report = run(&runner, InstallConsent::Declined);

    assert_ne!(
        outcome_of(&report, "core.sched.scx-lavd"),
        OutcomeKind::Applied
    );
    assert!(
        !runner
            .commands()
            .iter()
            .any(|command| command.contains("apt-get install")),
        "{:?}",
        runner.commands()
    );
}
