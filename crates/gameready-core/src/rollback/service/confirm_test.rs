use indoc::indoc;

use crate::improvement::Privilege;
use crate::infra::exec::MockRunner;
use crate::steam::{PriorBlock, PriorScalar};

use super::*;

const DROPIN: &str = "/etc/sysctl.d/99-gameready.conf";
const MAX_MAP_COUNT: &str = "/proc/sys/vm/max_map_count";
const SCHEDULER: &str = "/sys/block/nvme0n1/queue/scheduler";
const CONFIG: &str = "/steam/config/config.vdf";

fn delete_undo() -> Undo {
    Undo::DeleteFile {
        path: DROPIN.into(),
        expect_sha256: "abc".to_owned(),
        privilege: Privilege::Root,
    }
}

#[test]
fn a_file_the_undo_removed_confirms_when_it_is_gone() {
    let runner = MockRunner::new();

    assert_eq!(confirm(&delete_undo(), &runner), None);
}

#[test]
fn a_file_that_survived_a_reported_removal_is_reported_back() {
    // `rm` exiting zero and the file still being there is exactly the case a
    // trusted exit code would tell the user was fine.
    let runner = MockRunner::new().with_file(DROPIN, "vm.max_map_count = 1\n");

    let reason = confirm(&delete_undo(), &runner).expect("not confirmed");

    assert!(reason.contains(DROPIN), "{reason}");
}

fn sysctl_undo() -> Undo {
    Undo::SetSysctl {
        key: "vm.max_map_count".to_owned(),
        value: "65530".to_owned(),
    }
}

#[test]
fn a_sysctl_confirms_from_proc_rather_than_from_the_exit_code() {
    let runner = MockRunner::new().with_file(MAX_MAP_COUNT, "65530\n");

    assert_eq!(confirm(&sysctl_undo(), &runner), None);
}

#[test]
fn a_sysctl_that_did_not_move_is_reported_back() {
    // `sysctl -w` on a key a container masks exits zero and changes nothing.
    let runner = MockRunner::new().with_file(MAX_MAP_COUNT, "2147483642\n");

    let reason = confirm(&sysctl_undo(), &runner).expect("not confirmed");

    assert!(reason.contains("2147483642"), "{reason}");
    assert!(reason.contains("65530"), "{reason}");
}

#[test]
fn a_sysctl_that_cannot_be_read_is_not_called_a_failure() {
    // No /proc to read means the check has no opinion. Reporting a failure
    // there would turn an unreadable file into a fake rollback error.
    let runner = MockRunner::new();

    assert_eq!(confirm(&sysctl_undo(), &runner), None);
}

fn sysfs_undo() -> Undo {
    Undo::WriteSysfs {
        path: SCHEDULER.into(),
        value: "mq-deadline".to_owned(),
    }
}

#[test]
fn a_scheduler_confirms_from_the_bracketed_choice() {
    // The queue file lists every scheduler and brackets the live one.
    let runner = MockRunner::new().with_file(SCHEDULER, "none [mq-deadline] kyber\n");

    assert_eq!(confirm(&sysfs_undo(), &runner), None);
}

#[test]
fn a_scheduler_still_on_the_wrong_choice_is_reported_back() {
    let runner = MockRunner::new().with_file(SCHEDULER, "[none] mq-deadline kyber\n");

    let reason = confirm(&sysfs_undo(), &runner).expect("not confirmed");

    assert!(reason.contains("mq-deadline"), "{reason}");
}

fn steam_config() -> String {
    indoc! {r#"
        "InstallConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "CompatToolMapping"
                        {
                            "1422450"
                            {
                                "name"        "proton_experimental"
                            }
                        }
                    }
                }
            }
        }
    "#}
    .to_owned()
}

fn steam_undo() -> Undo {
    Undo::RestoreSteamConfig {
        path: CONFIG.into(),
        sections: vec![PriorSection {
            section: [
                "InstallConfigStore",
                "Software",
                "Valve",
                "Steam",
                "CompatToolMapping",
                "1422450",
            ]
            .iter()
            .map(|part| (*part).to_owned())
            .collect(),
            prior: PriorBlock::Present {
                entries: vec![PriorScalar {
                    key: "name".to_owned(),
                    value: Some("proton_experimental".to_owned()),
                }],
            },
        }],
    }
}

#[test]
fn a_steam_config_holding_the_recorded_value_confirms() {
    let runner = MockRunner::new().with_file(CONFIG, steam_config());

    assert_eq!(confirm(&steam_undo(), &runner), None);
}

#[test]
fn a_steam_config_still_holding_what_the_run_wrote_is_reported_back() {
    let still_written = steam_config().replace("proton_experimental", "GE-Proton11-3");
    let runner = MockRunner::new().with_file(CONFIG, &still_written);

    let reason = confirm(&steam_undo(), &runner).expect("not confirmed");

    assert!(reason.contains(CONFIG), "{reason}");
}

#[test]
fn packages_have_nothing_to_read_back() {
    let runner = MockRunner::new();
    let undo = Undo::ReportPackages {
        manager: "apt-get".to_owned(),
        installed: vec!["gamemode".to_owned()],
    };

    assert_eq!(confirm(&undo, &runner), None);
}
