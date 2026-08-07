use gameready_core::infra::exec::MockRunner;

use super::run;

/// A machine whose swappiness is low and whose swap layout is unknown, which is
/// enough for every step to answer without any of them being able to apply.
fn machine() -> MockRunner {
    MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n")
}

#[test]
fn no_step_named_lists_the_ones_there_are() {
    let text = run(&machine(), None).expect("listed");

    assert!(text.contains("core.sysctl.max-map-count"), "{text}");
    assert!(text.contains("core.memory.swappiness"), "{text}");
}

#[test]
fn a_named_step_is_explained_against_this_machine() {
    let text = run(&machine(), Some("core.sysctl.max-map-count")).expect("explained");

    assert!(text.contains("core.sysctl.max-map-count"), "{text}");
    assert!(text.contains("Why"), "{text}");
    assert!(text.contains("Would do"), "{text}");
    assert!(text.contains("2147483642"), "{text}");
}

#[test]
fn an_unknown_step_is_told_which_ids_exist() {
    // A typo in an id is the likeliest way to get here, and nobody remembers
    // the ids, so the error carries them rather than sending the user away.
    let failure = run(&machine(), Some("core.sysctl.max-map-conut")).expect_err("refused");
    let message = format!("{failure}");

    assert!(message.contains("max-map-conut"), "{message}");
    assert!(message.contains("core.sysctl.max-map-count"), "{message}");
}

#[test]
fn explain_changes_nothing() {
    let runner = machine();
    let before = runner.paths();

    let _ = run(&runner, Some("core.sysctl.max-map-count")).expect("explained");

    assert!(
        runner
            .commands()
            .iter()
            .all(|command| !command.starts_with("sudo ")),
        "explain asked for root: {:?}",
        runner.commands()
    );
    assert_eq!(runner.paths().len(), before.len(), "explain wrote a file");
}
