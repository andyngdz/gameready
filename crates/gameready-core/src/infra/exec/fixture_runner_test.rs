use std::path::Path;

use indoc::indoc;
use tempfile::TempDir;

use super::*;

const MANIFEST_FIXTURE: &str = indoc! {r#"
    binaries = ["pacman"]

    [[commands]]
    line = "uname -r"
    stdout = "6.12.0"
    "#};

/// A fixture holding one file and one command answer.
fn fixture() -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(dir.path().join("etc")).expect("etc");
    std::fs::write(dir.path().join("etc/os-release"), "ID=arch\n").expect("os-release");
    std::fs::write(dir.path().join("commands.toml"), MANIFEST_FIXTURE).expect("manifest");
    dir
}

#[test]
fn an_absolute_path_lands_inside_the_fixture() {
    let dir = fixture();
    let runner = FixtureRunner::open(dir.path()).expect("opened");

    assert_eq!(
        runner.resolve(Path::new("/etc/os-release")),
        dir.path().join("etc/os-release")
    );
}

#[test]
fn a_listed_command_answers_the_way_the_manifest_says() {
    let dir = fixture();
    let runner = FixtureRunner::open(dir.path()).expect("opened");

    assert_eq!(runner.answer("uname -r").stdout, "6.12.0");
}

#[test]
fn an_unlisted_command_succeeds_with_nothing_to_say() {
    // A fixture only carries the commands a test depends on, so everything else
    // has to have a harmless answer rather than being an error.
    let dir = fixture();
    let runner = FixtureRunner::open(dir.path()).expect("opened");

    let answer = runner.answer("some other command");
    assert_eq!(answer.code, 0);
    assert!(answer.stdout.is_empty());
}

#[test]
fn a_fixture_with_no_manifest_is_still_usable() {
    // A machine whose steps only read files needs no command table at all.
    let dir = TempDir::new().expect("temp dir");
    let runner = FixtureRunner::open(dir.path()).expect("opened");

    assert_eq!(runner.answer("uname -r").code, 0);
    assert!(!runner.has_binary("pacman"));
}

#[test]
fn a_manifest_that_does_not_parse_is_reported_rather_than_ignored() {
    // Silently treating it as empty would make a snapshot fail somewhere far
    // from the typo that caused it.
    let dir = TempDir::new().expect("temp dir");
    std::fs::write(dir.path().join("commands.toml"), "binaries = not-a-list\n").expect("manifest");

    let failure = FixtureRunner::open(dir.path());
    assert!(failure.is_err(), "a broken manifest was accepted");
}

#[test]
fn only_the_binaries_the_fixture_lists_are_on_its_path() {
    let dir = fixture();
    let runner = FixtureRunner::open(dir.path()).expect("opened");

    assert!(runner.has_binary("pacman"));
    assert!(!runner.has_binary("apt-get"));
}
