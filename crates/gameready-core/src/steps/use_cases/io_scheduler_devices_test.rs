use super::*;
use crate::infra::exec::MockRunner;

/// A fake `/sys/block` holding the given `(name, rotational, scheduler)` disks.
fn mock_with(devices: &[(&str, &str, &str)]) -> MockRunner {
    let mut runner = MockRunner::new();
    for (name, rotational, scheduler) in devices {
        runner = runner
            .with_file(
                format!("/sys/block/{name}/queue/rotational"),
                format!("{rotational}\n"),
            )
            .with_file(
                format!("/sys/block/{name}/queue/scheduler"),
                format!("{scheduler}\n"),
            );
    }
    runner
}

#[test]
fn an_nvme_disk_targets_none() {
    let runner = mock_with(&[("nvme0n1", "0", "[mq-deadline] none")]);
    let disks = scan_disks(&runner).expect("scans");

    assert_eq!(disks.len(), 1);
    let nvme = &disks[0];
    assert_eq!(nvme.name, "nvme0n1");
    assert_eq!(nvme.target, "none");
    assert_eq!(nvme.state.active, "mq-deadline");
    assert!(nvme.needs_change());
}

#[test]
fn a_rotational_disk_targets_bfq_and_an_ssd_targets_mq_deadline() {
    let runner = mock_with(&[
        ("sda", "1", "[none] mq-deadline bfq"),
        ("sdb", "0", "[none] mq-deadline bfq"),
    ]);
    let disks = scan_disks(&runner).expect("scans");

    let sda = disks.iter().find(|disk| disk.name == "sda").expect("sda");
    let sdb = disks.iter().find(|disk| disk.name == "sdb").expect("sdb");
    assert_eq!(sda.target, "bfq");
    assert_eq!(sdb.target, "mq-deadline");
}

#[test]
fn virtual_devices_are_left_out() {
    let runner = mock_with(&[
        ("nvme0n1", "0", "[none]"),
        ("loop0", "0", "[none]"),
        ("zram0", "0", "[none]"),
    ]);
    let disks = scan_disks(&runner).expect("scans");

    let names: Vec<&str> = disks.iter().map(|disk| disk.name.as_str()).collect();
    assert_eq!(names, ["nvme0n1"]);
}

#[test]
fn a_disk_with_no_readable_scheduler_is_skipped() {
    // The rotational flag is there but the scheduler attribute is not, so the
    // disk cannot be classified and must not appear rather than crash the scan.
    let runner = MockRunner::new().with_file("/sys/block/sda/queue/rotational", "0\n");
    let disks = scan_disks(&runner).expect("scans");
    assert!(disks.is_empty());
}

#[test]
fn a_disk_already_on_its_target_needs_no_change() {
    let runner = mock_with(&[("nvme0n1", "0", "[none] mq-deadline")]);
    let disks = scan_disks(&runner).expect("scans");
    assert!(!disks[0].needs_change());
}

#[test]
fn the_summary_names_only_the_disks_that_change() {
    let runner = mock_with(&[
        ("nvme0n1", "0", "[mq-deadline] none"),
        ("sdb", "0", "[none] mq-deadline"),
    ]);
    let disks = scan_disks(&runner).expect("scans");

    let line = summary(&disks);
    assert!(line.contains("nvme0n1 -> none"), "{line}");
    assert!(line.contains("sdb -> mq-deadline"), "{line}");
}
