use gameready_core::improvement::{Dependency, DependencyKind, ImprovementId, PackageSpec};
use gameready_core::run::{DependencyStatus, PreflightReport, ResolvedDependency, RunPlan};

use super::*;

fn resolved(
    name: &'static str,
    bytes: u64,
    status: DependencyStatus,
    what: &'static str,
    why: &'static str,
) -> ResolvedDependency {
    ResolvedDependency {
        dependency: Dependency::new(
            DependencyKind::Package {
                spec: PackageSpec::uniform(name, bytes),
            },
            what,
            why,
        ),
        wanted_by: vec![ImprovementId::from_static("test.a")],
        status,
    }
}

/// A step that installs a package itself, the way `core.pkg.tools` does.
fn self_install(package: &str, bytes: u64) -> (ImprovementId, PlannedInstall) {
    (
        ImprovementId::from_static("core.pkg.tools"),
        PlannedInstall {
            package: package.to_owned(),
            what: format!("what {package} is"),
            why: format!("why the run wants {package}"),
            approx_bytes: bytes,
        },
    )
}

fn plan_with(
    dependencies: Vec<ResolvedDependency>,
    total: u64,
    step_installs: Vec<(ImprovementId, PlannedInstall)>,
) -> RunPlan {
    plan_present(dependencies, total, step_installs, Vec::new())
}

fn plan_present(
    dependencies: Vec<ResolvedDependency>,
    total: u64,
    step_installs: Vec<(ImprovementId, PlannedInstall)>,
    step_present: Vec<String>,
) -> RunPlan {
    RunPlan {
        settled: Vec::new(),
        pending: Vec::new(),
        deferred: Vec::new(),
        contested: Vec::new(),
        preflight: PreflightReport {
            dependencies,
            total_install_bytes: total,
        },
        step_installs,
        step_present,
        started: std::time::Instant::now(),
    }
}

fn rendered(plan: &RunPlan) -> String {
    InstallList::new(plan, PackageManagerKind::Apt).to_string()
}

/// The screen without the blank line that separates it from the block above.
fn body(plan: &RunPlan) -> String {
    rendered(plan).trim_start().to_owned()
}

#[test]
fn every_package_names_itself_what_it_is_and_why_it_is_needed() {
    let plan = plan_with(
        vec![resolved(
            "mangohud",
            5_000_000,
            DependencyStatus::Missing,
            "an in-game frame-rate overlay",
            "so you can see whether any of this helped",
        )],
        5_000_000,
        Vec::new(),
    );
    let text = rendered(&plan);

    assert!(text.contains("mangohud"), "{text}");
    assert!(text.contains("an in-game frame-rate overlay"), "{text}");
    assert!(
        text.contains("so you can see whether any of this helped"),
        "{text}"
    );
}

#[test]
fn a_step_that_installs_its_own_packages_reaches_the_screen() {
    // The bug this covers: `core.pkg.tools` installs inside its own apply
    // rather than declaring a dependency, so a screen that read only the
    // pre-flight report asked about nothing while the run installed gamemode.
    let plan = plan_with(
        Vec::new(),
        0,
        vec![
            self_install("gamemode", 1_100_000),
            self_install("mangohud", 5_400_000),
        ],
    );
    let text = rendered(&plan);

    assert!(
        body(&plan).starts_with("2 packages to install · 6 MB"),
        "{text}"
    );
    assert!(text.contains("what gamemode is"), "{text}");
    assert!(text.contains("why the run wants gamemode"), "{text}");
}

#[test]
fn a_tool_the_machine_already_has_is_named_even_when_no_dependency_declared_it() {
    // core.pkg.tools is titled "install gamemode and mangohud". Fetching only
    // one of them without saying why reads as though the other went missing.
    let plan = plan_present(
        Vec::new(),
        0,
        vec![self_install("mangohud", 5_400_000)],
        vec!["gamemode".to_owned()],
    );
    let text = rendered(&plan);

    assert!(body(&plan).starts_with("1 package to install"), "{text}");
    assert!(text.contains("Already here: gamemode"), "{text}");
}

#[test]
fn a_run_with_no_size_estimate_does_not_claim_zero_megabytes() {
    let plan = plan_with(Vec::new(), 0, vec![self_install("gamemode", 0)]);
    let text = rendered(&plan);

    assert!(body(&plan).starts_with("1 package to install\n"), "{text}");
    assert!(!text.contains("0 MB"), "{text}");
}

#[test]
fn the_screen_says_packages_survive_a_rollback() {
    // The one consequence the user cannot undo later, so it has to be on the
    // screen where they agree to it.
    let plan = plan_with(Vec::new(), 0, vec![self_install("gamemode", 0)]);
    let text = rendered(&plan);

    assert!(text.contains("leaves packages installed"), "{text}");
}

#[test]
fn what_is_already_installed_is_listed_but_not_counted() {
    let plan = plan_with(
        vec![
            resolved(
                "mangohud",
                5_000_000,
                DependencyStatus::Missing,
                "an overlay",
                "to measure",
            ),
            resolved(
                "gamemode",
                2_000_000,
                DependencyStatus::Present,
                "a governor switcher",
                "to raise clocks while playing",
            ),
        ],
        5_000_000,
        Vec::new(),
    );
    let text = rendered(&plan);

    assert!(
        body(&plan).starts_with("1 package to install · 5 MB"),
        "{text}"
    );
    assert!(text.contains("Already here: gamemode"), "{text}");
}

#[test]
fn nothing_to_install_renders_nothing() {
    let plan = plan_with(
        vec![resolved(
            "gamemode",
            2_000_000,
            DependencyStatus::Present,
            "a governor switcher",
            "to raise clocks while playing",
        )],
        0,
        Vec::new(),
    );
    let text = rendered(&plan);

    assert!(text.is_empty(), "{text}");
}

#[test]
fn a_size_under_a_megabyte_does_not_round_to_zero() {
    assert_eq!(approx_size(400_000), "under 1 MB");
    assert_eq!(approx_size(5_000_000), "5 MB");
    assert_eq!(approx_size(900_000_000), "900 MB");
}

#[test]
fn the_screen_ends_on_its_own_line_so_the_summary_after_it_starts_on_a_new_one() {
    // The bug this covers: the last line was written with `write!`, so the
    // caller concatenating the summary onto it printed
    // "leaves packages installed.Nothing changed:" on one line.
    let plan = plan_with(Vec::new(), 0, vec![self_install("gamemode", 0)]);
    let text = rendered(&plan);

    assert!(text.ends_with('\n'), "{text}");
}
