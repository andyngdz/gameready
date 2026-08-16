use super::*;

#[test]
fn a_left_or_refused_undo_is_not_a_failure() {
    // Leaving a package installed is the designed behaviour, not an error, so
    // it must not push the rollback's exit code non-zero.
    assert!(!UndoOutcome::Left {
        reason: "kept".to_owned()
    }
    .is_failure());
    assert!(!UndoOutcome::Refused {
        reason: "edited".to_owned()
    }
    .is_failure());
    assert!(!UndoOutcome::AlreadyGone.is_failure());
    assert!(UndoOutcome::Failed {
        error: "boom".to_owned()
    }
    .is_failure());
}

#[test]
fn an_empty_plan_has_nothing_to_do() {
    let plan = RollbackPlan {
        run: crate::journal::RunId::generate(),
        undos: Vec::new(),
    };
    assert!(plan.is_empty());
}

fn plan_of(undos: Vec<Undo>) -> RollbackPlan {
    RollbackPlan {
        run: crate::journal::RunId::generate(),
        undos: undos
            .into_iter()
            .enumerate()
            .map(|(seq, undo)| PlannedUndo {
                step: ImprovementId::from_static("core.sysctl.max-map-count"),
                seq: seq as u64,
                undo,
            })
            .collect(),
    }
}

#[test]
fn a_run_that_only_touched_the_users_own_files_needs_no_password() {
    let plan = plan_of(vec![Undo::DeleteFile {
        path: "/home/someone/.config/gameready/demo.conf".into(),
        expect_sha256: "0".repeat(64),
        privilege: Privilege::User,
    }]);

    assert!(!plan.needs_root());
}

#[test]
fn one_system_change_in_the_run_is_enough_to_need_a_password() {
    let plan = plan_of(vec![
        Undo::DeleteFile {
            path: "/home/someone/.config/gameready/demo.conf".into(),
            expect_sha256: "0".repeat(64),
            privilege: Privilege::User,
        },
        Undo::SetSysctl {
            key: "vm.max_map_count".to_owned(),
            value: "65530".to_owned(),
        },
    ]);

    assert!(plan.needs_root());
}

#[test]
fn a_directory_made_in_the_users_own_home_needs_no_password_either() {
    // Regression: RemoveDirIfEmpty carried no privilege and was hardcoded to
    // root, so the first step to create a directory under $HOME made a
    // user-only run demand a password to undo itself.
    let plan = plan_of(vec![
        Undo::RemoveDirIfEmpty {
            path: "/home/someone/.config/environment.d".into(),
            privilege: Privilege::User,
        },
        Undo::DeleteFile {
            path: "/home/someone/.config/environment.d/99-gameready-shader-cache.conf".into(),
            expect_sha256: "0".repeat(64),
            privilege: Privilege::User,
        },
    ]);

    assert!(!plan.needs_root());
}

#[test]
fn a_directory_made_outside_the_home_still_needs_a_password() {
    let plan = plan_of(vec![Undo::RemoveDirIfEmpty {
        path: "/etc/gameready".into(),
        privilege: Privilege::Root,
    }]);

    assert!(plan.needs_root());
}

#[test]
fn reporting_packages_is_not_a_system_change() {
    // It performs nothing. Whether the caller then removes them is a policy the
    // command applies, not something this record says.
    let plan = plan_of(vec![Undo::ReportPackages {
        manager: "apt-get".to_owned(),
        installed: vec!["mangohud".to_owned()],
    }]);

    assert!(!plan.needs_root());
}

#[test]
fn a_plan_that_restores_steam_config_has_to_close_steam() {
    let plan = RollbackPlan {
        run: crate::journal::RunId::generate(),
        undos: vec![
            PlannedUndo {
                step: SteamLaunchOptions::id_const(),
                seq: 2,
                undo: Undo::RestoreSteamConfig {
                    path: "/steam/config/localconfig.vdf".into(),
                    sections: Vec::new(),
                },
            },
            PlannedUndo {
                step: SteamProton::id_const(),
                seq: 1,
                undo: Undo::RestoreSteamConfig {
                    path: "/steam/config/config.vdf".into(),
                    sections: Vec::new(),
                },
            },
        ],
    };

    assert!(plan.touches_steam());
}

#[test]
fn a_plan_without_a_steam_step_leaves_steam_alone() {
    let plan = plan_of(vec![Undo::SetSysctl {
        key: "vm.max_map_count".to_owned(),
        value: "65530".to_owned(),
    }]);

    assert!(!plan.touches_steam());
}
