use crate::improvement::Privilege;
use std::path::PathBuf;

use super::*;
use crate::journal::PriorUnitState;

#[test]
fn a_file_we_created_is_undone_by_deleting_it() {
    let change = Change::FileWritten {
        path: PathBuf::from("/etc/sysctl.d/99-gameready.conf"),
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
        other @ (Undo::RestoreSteamConfig { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::ReportPackages { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a delete, got {other:?}"),
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
        | Undo::RestoreSteamConfig { .. }
        | Undo::SetSysctl { .. }
        | Undo::WriteSysfs { .. }
        | Undo::RestoreUnit { .. }
        | Undo::RemoveDirIfEmpty { .. }
        | Undo::RemoveDirTree { .. }) => panic!("expected a report, got {other:?}"),
    }
}

#[test]
fn a_unit_is_undone_by_returning_it_to_its_prior_state() {
    let change = Change::SystemdUnit {
        unit: "tuned.service".to_owned(),
        was_enabled: false,
    };

    assert_eq!(
        change.inverse(),
        Undo::RestoreUnit {
            unit: "tuned.service".to_owned(),
            prior: PriorUnitState::WasDisabled,
        }
    );
}

#[test]
fn a_unit_that_was_already_enabled_records_that_it_should_stay() {
    let change = Change::SystemdUnit {
        unit: "tuned.service".to_owned(),
        was_enabled: true,
    };

    assert_eq!(
        change.inverse(),
        Undo::RestoreUnit {
            unit: "tuned.service".to_owned(),
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
