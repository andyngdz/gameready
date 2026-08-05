use indoc::indoc;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::infra::exec::MockRunner;

fn facts_kernel_7() -> SystemFacts {
    SystemFacts::fixture(Family::Debian)
}

fn facts_kernel_5() -> SystemFacts {
    SystemFacts::new(
        facts_kernel_7().distro,
        KernelVersion::new(5, 15, 0),
        "5.15.0-generic".to_owned(),
    )
}

#[test]
fn mitigations_off_is_flagged() {
    let runner = MockRunner::new()
        .with_file(
            CMDLINE_PATH,
            "BOOT_IMAGE=/boot/vmlinuz root=UUID=abc mitigations=off quiet",
        )
        .with_file(SWAPPINESS_PATH, "60\n");

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].finding.contains("mitigations=off"));
}

#[test]
fn mitigations_auto_is_not_flagged() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "BOOT_IMAGE=/boot/vmlinuz root=UUID=abc quiet")
        .with_file(SWAPPINESS_PATH, "60\n");

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert!(warnings.is_empty());
}

#[test]
fn swappiness_1_is_flagged() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "BOOT_IMAGE=/boot/vmlinuz quiet")
        .with_file(SWAPPINESS_PATH, "1\n");

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].finding.contains("swappiness"));
}

#[test]
fn swappiness_60_is_not_flagged() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "quiet")
        .with_file(SWAPPINESS_PATH, "60\n");

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert!(warnings.is_empty());
}

#[test]
fn dead_sysctl_in_conf_on_new_kernel_is_flagged() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "quiet")
        .with_file(SWAPPINESS_PATH, "60\n")
        .with_file(
            "/etc/sysctl.conf",
            indoc! {"
                # gaming tweaks
                kernel.sched_latency_ns = 1000000
                "},
        );

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].finding.contains("sched_latency_ns"));
}

#[test]
fn dead_sysctl_on_old_kernel_is_not_flagged() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "quiet")
        .with_file(SWAPPINESS_PATH, "60\n")
        .with_file(
            "/etc/sysctl.conf",
            indoc! {"
                kernel.sched_latency_ns = 1000000
                "},
        );

    let warnings = check_warnings(&facts_kernel_5(), &runner);

    assert!(warnings.is_empty());
}

#[test]
fn commented_sysctl_line_is_not_flagged() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "quiet")
        .with_file(SWAPPINESS_PATH, "60\n")
        .with_file(
            "/etc/sysctl.conf",
            indoc! {"
                # kernel.sched_latency_ns = 1000000
                "},
        );

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert!(warnings.is_empty());
}

#[test]
fn multiple_warnings_can_fire_together() {
    let runner = MockRunner::new()
        .with_file(CMDLINE_PATH, "mitigations=off quiet")
        .with_file(SWAPPINESS_PATH, "1\n")
        .with_file(
            "/etc/sysctl.conf",
            indoc! {"
                kernel.sched_min_granularity_ns = 500000
                "},
        );

    let warnings = check_warnings(&facts_kernel_7(), &runner);

    assert_eq!(warnings.len(), 3);
}
