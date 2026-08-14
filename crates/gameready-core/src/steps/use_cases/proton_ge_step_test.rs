use std::path::PathBuf;

use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::improvement::{ApplyCx, CoreCx, CoreImprovement, Improvement, Privilege, Probe, Tag};
use crate::infra::exec::MockRunner;
use crate::journal::{Change, Journal, RunId, StatePaths};
use crate::steps::constants::PROTON_GE_LATEST_URL;

const COMPAT: &str = "/home/test/.steam/root/compatibilitytools.d";
const TAG: &str = "GE-Proton11-5";

/// What the release names the x86_64 tarball, which is not the tag alone.
const TARBALL: &str = "GE-Proton11-5-x86_64.tar.gz";

/// The directory that tarball extracts to, which is what Steam sees.
const INSTALL: &str = "GE-Proton11-5-x86_64";

fn release_json() -> &'static str {
    indoc! {r#"
        {
          "tag_name": "GE-Proton11-5",
          "assets": [
            {
              "name": "GE-Proton11-5-x86_64.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz"
            },
            {
              "name": "GE-Proton11-5-x86_64.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-5-x86_64.sha512sum"
            }
          ]
        }
    "#}
}

fn step() -> ProtonGe {
    ProtonGe::with_compat_dir(PathBuf::from(COMPAT))
}

fn base_runner() -> MockRunner {
    MockRunner::new()
        .with_binary("curl")
        .with_binary("tar")
        .with_binary("sha512sum")
        .with_file("/home/test/.steam/root", "")
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), release_json())
}

fn facts() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

fn journal(dir: &TempDir) -> Journal {
    Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate())
        .expect("journal opens")
}

fn apply_runner() -> MockRunner {
    let temp_tarball = format!("{}/{TARBALL}", std::env::temp_dir().display());
    base_runner()
        .answering(
            "curl -sfL https://github.com/dl/GE-Proton11-5-x86_64.sha512sum",
            format!("abc123  {TARBALL}\n"),
        )
        .answering(
            format!("sha512sum {temp_tarball}"),
            format!("abc123  {TARBALL}\n"),
        )
        .serving(
            "https://github.com/dl/GE-Proton11-5-x86_64.tar.gz",
            "a tarball, as far as this test is concerned",
        )
}

// --- identity ---

#[test]
fn id_is_stable() {
    assert_eq!(step().id().as_str(), "core.proton.ge");
}

#[test]
fn privilege_is_user() {
    assert_eq!(step().privilege(), Privilege::User);
}

#[test]
fn tags_include_steam() {
    assert!(step().tags().contains(&Tag::Steam));
}

// --- probe ---

#[test]
fn probe_not_applicable_when_curl_missing() {
    let runner = MockRunner::new();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    assert!(matches!(
        step().probe(&cx).expect("ok"),
        Probe::NotApplicable { .. }
    ));
}

#[test]
fn probe_not_applicable_when_steam_root_missing() {
    let runner = MockRunner::new().with_binary("curl");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    assert!(matches!(
        step().probe(&cx).expect("ok"),
        Probe::NotApplicable { .. }
    ));
}

#[test]
fn probe_already_applied_when_tag_dir_exists() {
    let runner = base_runner()
        .with_file(format!("{COMPAT}/{INSTALL}"), "")
        .with_file(
            format!("{COMPAT}/{INSTALL}/compatibilitytool.vdf"),
            "manifest",
        );
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    assert!(matches!(
        step().probe(&cx).expect("ok"),
        Probe::AlreadyApplied { .. }
    ));
}

#[test]
fn probe_applicable_when_tag_dir_absent() {
    let runner = base_runner();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    assert!(matches!(step().probe(&cx).expect("ok"), Probe::Applicable));
}

#[test]
fn probe_update_available_when_an_older_ge_install_exists() {
    let runner = base_runner().with_file(
        format!("{COMPAT}/GE-Proton11-2/compatibilitytool.vdf"),
        "manifest",
    );
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let found = step().probe(&cx).expect("ok");
    assert!(matches!(
        found,
        Probe::UpdateAvailable {
            installed,
            latest
        } if installed == "GE-Proton11-2" && latest == TAG
    ));
}

#[test]
fn probe_names_the_newest_install_rather_than_the_first_one_listed() {
    // The listing arrives sorted as text, where GE-Proton10-15 comes first and
    // GE-Proton9-20 comes last. Reporting either one tells the user they are on
    // a build two releases older than what they actually have.
    let runner = base_runner()
        .with_file(
            format!("{COMPAT}/GE-Proton10-15/compatibilitytool.vdf"),
            "m",
        )
        .with_file(format!("{COMPAT}/GE-Proton11-2/compatibilitytool.vdf"), "m")
        .with_file(format!("{COMPAT}/GE-Proton9-20/compatibilitytool.vdf"), "m");
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);

    let found = step().probe(&cx).expect("ok");

    assert!(
        matches!(
            &found,
            Probe::UpdateAvailable { installed, .. } if installed == "GE-Proton11-2"
        ),
        "{found:?}"
    );
}

#[test]
fn probe_applicable_when_only_an_unrelated_compat_tool_is_installed() {
    let runner = base_runner().with_file(
        format!("{COMPAT}/DXVK-Custom/compatibilitytool.vdf"),
        "manifest",
    );
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    assert!(matches!(step().probe(&cx).expect("ok"), Probe::Applicable));
}

#[test]
fn probe_not_applicable_when_github_unreachable() {
    let runner = MockRunner::new()
        .with_binary("curl")
        .with_file("/home/test/.steam/root", "")
        .failing(format!("curl -sfL {PROTON_GE_LATEST_URL}"));
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    assert!(matches!(
        step().probe(&cx).expect("ok"),
        Probe::NotApplicable { .. }
    ));
}

// --- plan ---

#[test]
fn plan_describes_the_tag() {
    let runner = base_runner();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let plan = step().plan(&cx).expect("plan succeeds");
    assert!(plan.summary.contains(TAG));
}

// --- apply ---

#[test]
fn apply_records_dir_tree_installed() {
    let runner = apply_runner();
    let facts = facts();
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        ProtonGe::id_const(),
        &runner,
        &mut log,
    );

    step().apply(&mut cx).expect("apply succeeds");

    let changes = cx.recorded();
    assert_eq!(changes.len(), 1);
    match &changes[0] {
        Change::DirTreeInstalled { path, privilege } => {
            assert_eq!(path, &PathBuf::from(format!("{COMPAT}/{INSTALL}")));
            assert_eq!(*privilege, Privilege::User);
        }
        other @ (Change::FileWritten { .. }
        | Change::SteamConfigWritten { .. }
        | Change::FileRemoved { .. }
        | Change::SysctlRuntime { .. }
        | Change::SysfsWrite { .. }
        | Change::PackagesInstalled { .. }
        | Change::SystemdUnit { .. }
        | Change::DirCreated { .. }) => panic!("expected DirTreeInstalled, got {other:?}"),
    }
}

#[test]
fn apply_runs_tar_to_extract() {
    let runner = apply_runner();
    let facts = facts();
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        ProtonGe::id_const(),
        &runner,
        &mut log,
    );

    step().apply(&mut cx).expect("apply succeeds");

    let commands = runner.commands();
    assert!(
        commands
            .iter()
            .any(|cmd| cmd.contains("tar") && cmd.contains(COMPAT)),
        "expected a tar command extracting to compat dir, got: {commands:?}"
    );
}

// --- verify ---

#[test]
fn verify_passes_when_vdf_exists() {
    let runner = base_runner().with_file(
        format!("{COMPAT}/{INSTALL}/compatibilitytool.vdf"),
        "manifest",
    );
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let verification = step().verify(&cx).expect("verify succeeds");
    assert!(verification.passed());
}

#[test]
fn verify_fails_when_vdf_absent() {
    let runner = base_runner();
    let facts = facts();
    let cx = CoreCx::new(&facts, &runner);
    let verification = step().verify(&cx).expect("verify succeeds");
    assert!(!verification.passed());
}

// --- rollback ---

#[test]
fn rollback_removes_the_installed_dir_tree() {
    let runner = base_runner();
    let facts = facts();
    let dir = TempDir::new().expect("temp dir");
    let mut log = journal(&dir);
    let undo = vec![Change::DirTreeInstalled {
        path: PathBuf::from(format!("{COMPAT}/{INSTALL}")),
        privilege: Privilege::User,
    }];
    let mut cx = ApplyCx::new(
        CoreCx::new(&facts, &runner),
        ProtonGe::id_const(),
        &runner,
        &mut log,
    );

    step().rollback(&undo, &mut cx).expect("rollback succeeds");

    let commands = runner.commands();
    assert!(
        commands
            .iter()
            .any(|cmd| cmd.contains("rm") && cmd.contains(TAG)),
        "expected rm -rf of the tag dir, got: {commands:?}"
    );
}
