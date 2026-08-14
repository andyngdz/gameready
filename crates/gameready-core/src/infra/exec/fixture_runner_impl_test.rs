use indoc::indoc;
use tempfile::TempDir;

use super::*;

const MANIFEST_FIXTURE: &str = indoc! {r#"
    binaries = ["pacman"]

    [[commands]]
    line = "uname -r"
    stdout = "6.12.0"

    [[commands]]
    line = "pacman -Q mangohud"
    code = 1
    stderr = "package not found"
    "#};

fn fixture() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path();
    std::fs::create_dir_all(root.join("sys/block/nvme0n1")).expect("sys");
    std::fs::create_dir_all(root.join("sys/block/sda")).expect("sys");
    std::fs::create_dir_all(root.join("etc")).expect("etc");
    std::fs::write(root.join("etc/os-release"), "ID=arch\n").expect("os-release");
    std::fs::write(root.join("commands.toml"), MANIFEST_FIXTURE).expect("manifest");
    dir
}

fn runner(dir: &TempDir) -> FixtureRunner {
    FixtureRunner::open(dir.path()).expect("opened")
}

#[test]
fn a_file_reads_out_of_the_fixture_rather_than_the_real_root() {
    let dir = fixture();

    let text = runner(&dir)
        .read_to_string(Path::new("/etc/os-release"))
        .expect("read");

    assert_eq!(text, "ID=arch\n");
}

#[test]
fn a_missing_file_names_the_path_the_caller_asked_for() {
    // Naming the fixture directory instead would read as a bug in gameready
    // rather than as a gap in the fake machine.
    let dir = fixture();

    let failure = runner(&dir)
        .read_to_string(Path::new("/proc/sys/vm/swappiness"))
        .expect_err("missing");

    assert!(
        format!("{failure}").contains("/proc/sys/vm/swappiness"),
        "{failure}"
    );
}

#[test]
fn a_listing_comes_back_as_paths_on_the_fake_machine() {
    let dir = fixture();

    let entries = runner(&dir)
        .read_dir(Path::new("/sys/block"))
        .expect("read");

    assert_eq!(
        entries,
        vec![
            PathBuf::from("/sys/block/nvme0n1"),
            PathBuf::from("/sys/block/sda"),
        ]
    );
}

#[test]
fn a_command_that_the_manifest_fails_is_an_error_to_run() {
    let dir = fixture();
    let cmd = Cmd::user("pacman").arg("-Q").arg("mangohud");

    let failure = runner(&dir).run(&cmd).expect_err("non-zero");

    assert!(matches!(failure, ExecError::NonZeroExit { code: 1, .. }));
}

#[test]
fn the_same_command_is_an_answer_when_failure_is_allowed() {
    // A package query exiting non-zero is how "not installed" is expressed.
    let dir = fixture();
    let cmd = Cmd::user("pacman").arg("-Q").arg("mangohud");

    let output = runner(&dir).run_allowing_failure(&cmd).expect("answered");

    assert_eq!(output.code, 1);
}

#[test]
fn every_write_is_refused_so_a_fixture_run_cannot_reach_a_disk() {
    let dir = fixture();
    let runner = runner(&dir);
    let path = Path::new("/etc/sysctl.d/99-gameready.conf");

    assert!(runner.write_file(path, "x", Privilege::Root).is_err());
    assert!(runner.write_sysfs(path, "x", Privilege::Root).is_err());
    assert!(runner.remove_file(path, Privilege::Root).is_err());
    assert!(!dir.path().join("etc/sysctl.d").exists());
}

#[test]
fn which_answers_from_the_manifest_rather_than_the_real_path() {
    let dir = fixture();
    let runner = runner(&dir);

    assert!(runner.which("pacman").is_some());
    assert!(runner.which("apt-get").is_none());
}
