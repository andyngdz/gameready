//! The pending-side half of re-probing, made executable.
//!
//! The CPU governor step probes as applicable on a machine with no gamemode.
//! But this same run is about to install gamemode, which raises the governor per
//! game and makes pinning it system-wide the wrong move. So the governor step is
//! looked at once more, right before it would apply, and steps aside. This is
//! the mock's `governor left to gamemode, ran after the re-check` line.

// An integration test is its own crate, so the crate-level allow in lib.rs does
// not reach here. A test reports failure by panicking either way.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use gameready_core::facts::{Family, SystemFacts};
use gameready_core::improvement::{CoreCx, CoreImprovement, OutcomeKind};
use gameready_core::infra::exec::MockRunner;
use gameready_core::infra::pkg::Apt;
use gameready_core::journal::{Journal, RunId, StatePaths};
use gameready_core::run::{execute, InstallConsent, Mode, RunReport};
use gameready_core::steps::{CpuGovernor, GamingTools};
use tempfile::TempDir;

const POLICY0_GOVERNOR: &str = "/sys/devices/system/cpu/cpufreq/policy0/scaling_governor";
const POLICY0_AVAILABLE: &str =
    "/sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors";
const INSTALL_TOOLS: &str = "sudo apt-get install --yes --no-install-recommends gamemode mangohud";

/// A machine on powersave with no gamemode, and an apt that will install it.
///
/// Installing the tools is what puts `gamemoded` on PATH, which is the same
/// signal the tools step and the governor step both read.
fn machine_without_gamemode() -> MockRunner {
    MockRunner::new()
        .with_file(POLICY0_GOVERNOR, "powersave\n")
        .with_file(POLICY0_AVAILABLE, "performance powersave\n")
        .failing("dpkg-query --showformat=${Version} --show gamemode")
        .failing("dpkg-query --showformat=${Version} --show mangohud")
        .answering("apt-cache show gamemode", "Package: gamemode\n")
        .answering("apt-cache show mangohud", "Package: mangohud\n")
        .where_command_adds_binary(INSTALL_TOOLS, "gamemoded")
        .where_command_adds_binary(INSTALL_TOOLS, "mangohud")
}

fn run(runner: &MockRunner) -> RunReport {
    let dir = TempDir::new().expect("temp dir");
    let facts = SystemFacts::fixture(Family::Debian);
    let packages = Apt;
    let cx = CoreCx::new(&facts, runner).with_packages(&packages);
    let mut journal = Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens");

    let steps: Vec<Box<dyn CoreImprovement>> = vec![Box::new(GamingTools), Box::new(CpuGovernor)];
    execute(
        steps,
        &cx,
        &mut journal,
        Mode::Apply,
        InstallConsent::Granted,
        &mut |_| {},
    )
    .expect("the run completes")
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
fn gamemode_arriving_mid_run_takes_the_governor_step_off_the_list() {
    let runner = machine_without_gamemode();
    let report = run(&runner);

    assert_eq!(
        outcome_of(&report, "core.pkg.tools"),
        OutcomeKind::Applied,
        "the tools step should have installed gamemode"
    );
    assert_eq!(
        outcome_of(&report, "core.cpu.governor"),
        OutcomeKind::AlreadySet,
        "the governor step should defer to gamemode after the re-check"
    );
}

#[test]
fn the_governor_step_writes_nothing_once_gamemode_is_there() {
    let runner = machine_without_gamemode();
    run(&runner);

    assert!(
        !runner
            .commands()
            .iter()
            .any(|command| command.contains("scaling_governor")),
        "{:?}",
        runner.commands()
    );
}
