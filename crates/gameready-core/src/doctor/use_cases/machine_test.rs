use super::*;
use crate::infra::exec::MockRunner;
use crate::steps::domain::SwapBacking;

#[test]
fn a_bare_machine_reports_no_swap_no_disks() {
    let report = machine_report(&MockRunner::new());
    assert!(report.swap.is_none());
    assert!(report.disks.is_empty());
}

#[test]
fn zram_swap_reports_its_backing_and_total_size() {
    let runner = MockRunner::new().with_file(
        "/proc/swaps",
        indoc::indoc! {"
            Filename Type Size Used Priority
            /dev/zram0 partition 16777216 0 100
        "},
    );
    let swap = machine_report(&runner).swap.expect("swap read");
    assert_eq!(swap.backing, SwapBacking::Zram);
    assert_eq!(swap.total_kib, 16_777_216);
}

#[test]
fn a_disk_reports_its_active_scheduler() {
    let runner = MockRunner::new()
        .with_file("/sys/block/nvme0n1/queue/rotational", "0\n")
        .with_file("/sys/block/nvme0n1/queue/scheduler", "[none] mq-deadline\n");
    let disks = machine_report(&runner).disks;
    assert_eq!(disks.len(), 1);
    let disk = &disks[0];
    assert_eq!(disk.name, "nvme0n1");
    assert_eq!(disk.scheduler, "none");
}
