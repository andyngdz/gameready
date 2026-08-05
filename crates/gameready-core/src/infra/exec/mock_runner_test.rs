use std::path::Path;

use super::*;
use crate::exec::Cmd;

#[test]
fn records_every_command_in_order() {
    let runner = MockRunner::new();
    runner.run(&Cmd::user("uname").arg("-r")).expect("runs");
    runner
        .run(&Cmd::root("sysctl").arg("-w").arg("a=b"))
        .expect("runs");

    assert_eq!(runner.commands(), ["uname -r", "sudo sysctl -w a=b"]);
}

#[test]
fn answers_a_seeded_command() {
    let runner = MockRunner::new().answering("uname -r", "7.0.0-29-generic\n");
    let output = runner.run(&Cmd::user("uname").arg("-r")).expect("runs");
    assert_eq!(output.stdout_trimmed(), "7.0.0-29-generic");
}

#[test]
fn writes_land_in_the_fake_filesystem() {
    let runner = MockRunner::new();
    runner
        .write_file(Path::new("/etc/x.conf"), "value = 1", Privilege::Root)
        .expect("writes");

    assert_eq!(runner.file("/etc/x.conf").as_deref(), Some("value = 1"));
    assert!(runner.path_exists(Path::new("/etc/x.conf")));
}

#[test]
fn removing_a_file_that_is_already_gone_succeeds() {
    // Rollback must be safe to re-run after a partial undo.
    let runner = MockRunner::new();
    runner
        .remove_file(Path::new("/etc/absent.conf"), Privilege::Root)
        .expect("already gone is success");
}

#[test]
fn failing_at_an_index_fails_exactly_that_command() {
    let runner = MockRunner::new().failing_at(1);
    runner.run(&Cmd::user("first")).expect("first succeeds");
    let error = runner.run(&Cmd::user("second")).expect_err("second fails");
    assert!(error.to_string().contains("second"));
    runner
        .run(&Cmd::user("third"))
        .expect("third succeeds again");
}

#[test]
fn which_answers_only_for_seeded_binaries() {
    let runner = MockRunner::new().with_binary("sudo");
    assert!(runner.which("sudo").is_some());
    assert!(runner.which("clang").is_none());
}
