use gameready_core::infra::exec::MockRunner;

use super::run;

#[test]
fn reports_the_kernel_and_every_step() {
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");

    let text = run(&runner).expect("doctor reads the system");

    assert!(text.contains("7.0.0-29-generic"));
    assert!(text.contains("Ubuntu 26.04 LTS"));
    assert!(
        text.contains("apt-get"),
        "names the package manager: {text}"
    );
    assert!(text.contains("core.sysctl.max-map-count"));
    assert!(text.contains("would apply"));
}

#[test]
fn says_already_set_when_the_value_is_high_enough() {
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "2147483642\n");

    let text = run(&runner).expect("doctor reads the system");
    assert!(text.contains("already set"));
}

#[test]
fn doctor_changes_nothing() {
    let runner = MockRunner::new()
        .answering("uname -r", "7.0.0-29-generic\n")
        .with_file(
            "/etc/os-release",
            super::super::constants::OS_RELEASE_FIXTURE,
        )
        .with_file("/proc/sys/vm/max_map_count", "1048576\n");

    let before = runner.paths();
    let _ = run(&runner).expect("doctor reads the system");

    // Doctor probes with whatever read-only queries its steps need, and the
    // list grows as steps are added. What must never change is that none of
    // them takes privilege or leaves a file behind.
    assert!(
        runner
            .commands()
            .iter()
            .all(|cmd| !cmd.starts_with("sudo ")),
        "doctor asked for root: {:?}",
        runner.commands()
    );
    assert_eq!(runner.paths().len(), before.len(), "doctor wrote a file");
}
