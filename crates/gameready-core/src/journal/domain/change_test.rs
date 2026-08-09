use crate::improvement::Privilege;
use std::path::PathBuf;

use super::*;
use crate::journal::PriorUnitState;

#[test]
fn a_file_we_created_is_undone_by_deleting_it() {
    let change = Change::FileWritten {
        path: PathBuf::from("/etc/sysctl.d/99-gameready.conf"),
        existed: false,
        backup: None,
        sha256_after: "abc123".to_owned(),
        mode: 0o644,
        privilege: Privilege::Root,
    };

    match change.inverse() {
        Undo::DeleteFile {
            path,
            expect_sha256,
            ..
        } => {
            assert_eq!(path, PathBuf::from("/etc/sysctl.d/99-gameready.conf"));
            // The digest lets rollback refuse to clobber a file the user edited.
            assert_eq!(expect_sha256, "abc123");
        }
        other @ (Undo::RestoreFile { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::ReportPackages { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveAptRepository { .. }
        | Undo::RestoreScxScheduler { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a delete, got {other:?}"),
    }
}

#[test]
fn a_file_we_replaced_is_undone_by_restoring_its_pre_image() {
    let change = Change::FileWritten {
        path: PathBuf::from("/home/u/.steam/config/localconfig.vdf"),
        existed: true,
        backup: Some(PathBuf::from("/state/backups/1/localconfig.vdf")),
        sha256_after: "def456".to_owned(),
        mode: 0o600,
        privilege: Privilege::Root,
    };

    match change.inverse() {
        Undo::RestoreFile { from, mode, .. } => {
            assert_eq!(from, PathBuf::from("/state/backups/1/localconfig.vdf"));
            // Mode comes from the record, never from the backup file itself.
            assert_eq!(mode, 0o600);
        }
        other @ (Undo::DeleteFile { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::ReportPackages { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveAptRepository { .. }
        | Undo::RestoreScxScheduler { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a restore, got {other:?}"),
    }
}

#[test]
fn a_sysctl_is_undone_by_writing_the_previous_value_back() {
    let change = Change::SysctlRuntime {
        key: "vm.max_map_count".to_owned(),
        previous: "1048576".to_owned(),
    };

    assert_eq!(
        change.inverse(),
        Undo::SetSysctl {
            key: "vm.max_map_count".to_owned(),
            value: "1048576".to_owned(),
        }
    );
}

#[test]
fn installed_packages_are_reported_rather_than_removed() {
    // Uninstalling is not the inverse of installing: dependency cascades and
    // other users relying on the package make removal a different operation.
    let change = Change::PackagesInstalled {
        manager: "apt".to_owned(),
        requested: vec!["gamemode".to_owned(), "mangohud".to_owned()],
        newly_installed: vec!["mangohud".to_owned()],
    };

    match change.inverse() {
        Undo::ReportPackages { installed, .. } => {
            // Only what we actually installed, not everything we asked for.
            assert_eq!(installed, ["mangohud"]);
        }
        other @ (Undo::DeleteFile { .. }
        | Undo::RestoreFile { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveAptRepository { .. }
        | Undo::RestoreScxScheduler { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a report, got {other:?}"),
    }
}

#[test]
fn a_unit_is_undone_by_returning_it_to_its_prior_state() {
    let change = Change::SystemdUnit {
        unit: "scx_loader.service".to_owned(),
        was_enabled: false,
        was_active: false,
    };

    assert_eq!(
        change.inverse(),
        Undo::RestoreUnit {
            unit: "scx_loader.service".to_owned(),
            prior: PriorUnitState::WasDisabled,
        }
    );
}

#[test]
fn a_unit_that_was_already_enabled_records_that_it_should_stay() {
    let change = Change::SystemdUnit {
        unit: "scx_loader.service".to_owned(),
        was_enabled: true,
        was_active: true,
    };

    assert_eq!(
        change.inverse(),
        Undo::RestoreUnit {
            unit: "scx_loader.service".to_owned(),
            prior: PriorUnitState::WasEnabled,
        }
    );
}

#[test]
fn a_dir_tree_installed_is_undone_by_recursive_removal() {
    let change = Change::DirTreeInstalled {
        path: PathBuf::from("/home/u/.steam/root/compatibilitytools.d/GE-Proton11-3"),
        privilege: Privilege::User,
    };

    assert_eq!(
        change.inverse(),
        Undo::RemoveDirTree {
            path: PathBuf::from("/home/u/.steam/root/compatibilitytools.d/GE-Proton11-3"),
            privilege: Privilege::User,
        }
    );
}

#[test]
fn every_change_round_trips_through_json() {
    // The journal is the undo record; a change that cannot be read back is a
    // change that cannot be undone after a crash.
    let changes = vec![
        Change::SysctlRuntime {
            key: "vm.swappiness".to_owned(),
            previous: "60".to_owned(),
        },
        Change::DirCreated {
            path: PathBuf::from("/etc/gameready"),
            privilege: Privilege::Root,
        },
        Change::DirTreeInstalled {
            path: PathBuf::from("/home/u/.steam/root/compatibilitytools.d/GE-Proton11-3"),
            privilege: Privilege::User,
        },
    ];

    for change in changes {
        let encoded = serde_json::to_string(&change).expect("encodes");
        let decoded: Change = serde_json::from_str(&encoded).expect("decodes");
        assert_eq!(decoded, change);
    }
}

#[test]
fn a_loaded_scheduler_is_undone_by_handing_the_cpu_back() {
    // No previous scheduler means the kernel was running its own, so the undo
    // is to stop rather than to switch to something.
    let change = Change::ScxScheduler { previous: None };

    match change.inverse() {
        Undo::RestoreScxScheduler { previous } => assert_eq!(previous, None),
        other @ (Undo::DeleteFile { .. }
        | Undo::RestoreFile { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::ReportPackages { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveAptRepository { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a scheduler restore, got {other:?}"),
    }
}

#[test]
fn a_scheduler_that_replaced_another_is_undone_by_switching_back() {
    let change = Change::ScxScheduler {
        previous: Some("bpfland".to_owned()),
    };

    match change.inverse() {
        Undo::RestoreScxScheduler { previous } => {
            assert_eq!(previous.as_deref(), Some("bpfland"));
        }
        other @ (Undo::DeleteFile { .. }
        | Undo::RestoreFile { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::ReportPackages { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveAptRepository { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a scheduler restore, got {other:?}"),
    }
}

#[test]
fn a_repository_we_added_is_undone_by_removing_it() {
    // The spec is recorded rather than the files it wrote, because the tool
    // that adds a PPA is the only thing that reliably knows which those are.
    let change = Change::AptRepository {
        spec: "ppa:arighi/sched-ext".to_owned(),
    };

    match change.inverse() {
        Undo::RemoveAptRepository { spec } => assert_eq!(spec, "ppa:arighi/sched-ext"),
        other @ (Undo::DeleteFile { .. }
        | Undo::RestoreFile { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::ReportPackages { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RestoreScxScheduler { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a repository removal, got {other:?}"),
    }
}
