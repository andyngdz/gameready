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
const TAG: &str = "GE-Proton11-3";

fn release_json() -> &'static str {
    indoc! {r#"
        {
          "tag_name": "GE-Proton11-3",
          "assets": [
            {
              "name": "GE-Proton11-3.tar.gz",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3.tar.gz"
            },
            {
              "name": "GE-Proton11-3.sha512sum",
              "browser_download_url": "https://github.com/dl/GE-Proton11-3.sha512sum"
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
    let temp_tarball = format!("{}/GE-Proton11-3.tar.gz", std::env::temp_dir().display());
    base_runner()
        .answering(
            "curl -sfL https://github.com/dl/GE-Proton11-3.sha512sum",
            "abc123  GE-Proton11-3.tar.gz\n",
        )
        .answering(
            format!("sha512sum {temp_tarball}"),
            "abc123  GE-Proton11-3.tar.gz\n",
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
        .with_file(format!("{COMPAT}/{TAG}"), "")
        .with_file(format!("{COMPAT}/{TAG}/compatibilitytool.vdf"), "manifest");
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
            assert_eq!(path, &PathBuf::from(format!("{COMPAT}/{TAG}")));
            assert_eq!(*privilege, Privilege::User);
        }
        other => panic!("expected DirTreeInstalled, got {other:?}"),
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
    let runner =
        base_runner().with_file(format!("{COMPAT}/{TAG}/compatibilitytool.vdf"), "manifest");
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
        path: PathBuf::from(format!("{COMPAT}/{TAG}")),
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
