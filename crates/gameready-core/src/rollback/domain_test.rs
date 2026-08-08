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
fn reporting_packages_is_not_a_system_change() {
    // It performs nothing. Whether the caller then removes them is a policy the
    // command applies, not something this record says.
    let plan = plan_of(vec![Undo::ReportPackages {
        manager: "apt-get".to_owned(),
        installed: vec!["mangohud".to_owned()],
    }]);

    assert!(!plan.needs_root());
}
