use super::*;

fn device(name: &str, rotational: bool) -> BlockDevice {
    BlockDevice {
        name: name.to_owned(),
        rotational,
    }
}

#[test]
fn an_nvme_drive_gets_no_scheduler() {
    assert_eq!(device("nvme0n1", false).target_scheduler(), "none");
}

#[test]
fn a_spinning_disk_gets_bfq() {
    assert_eq!(device("sda", true).target_scheduler(), "bfq");
}

#[test]
fn a_sata_ssd_gets_mq_deadline() {
    assert_eq!(device("sdb", false).target_scheduler(), "mq-deadline");
}

#[test]
fn an_nvme_drive_reporting_rotational_still_gets_none() {
    // The name wins: nothing spins an NVMe, and a stray rotational=1 must not
    // send it to bfq.
    assert_eq!(device("nvme1n1", true).target_scheduler(), "none");
}

#[test]
fn real_disks_are_tunable_and_virtual_devices_are_not() {
    assert!(is_tunable_disk("nvme0n1"));
    assert!(is_tunable_disk("sda"));
    for virtual_name in ["loop0", "ram0", "zram0", "dm-0", "md0", "sr0", "nbd0"] {
        assert!(!is_tunable_disk(virtual_name), "{virtual_name} is virtual");
    }
}

#[test]
fn the_scheduler_line_splits_into_active_and_available() {
    let state = parse_scheduler_line("none [mq-deadline] kyber bfq").expect("parses");
    assert_eq!(state.active, "mq-deadline");
    assert_eq!(state.available, ["none", "mq-deadline", "kyber", "bfq"]);
    assert!(state.offers("bfq"));
    assert!(!state.offers("none-such"));
}

#[test]
fn a_single_bracketed_scheduler_parses() {
    let state = parse_scheduler_line("[none]").expect("parses");
    assert_eq!(state.active, "none");
    assert_eq!(state.available, ["none"]);
}

#[test]
fn a_line_with_nothing_bracketed_is_not_a_scheduler_line() {
    assert!(parse_scheduler_line("none mq-deadline").is_none());
    assert!(parse_scheduler_line("").is_none());
}
