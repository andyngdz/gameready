use gameready_core::exec::Cmd;
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

    let _ = run(&runner).expect("doctor reads the system");

    assert!(
        runner
            .commands()
            .iter()
            .all(|cmd| cmd == &Cmd::user("uname").arg("-r").to_string()),
        "doctor ran something other than a probe: {:?}",
        runner.commands()
    );
}
