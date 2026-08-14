use super::*;

#[test]
fn only_the_named_scheduler_counts_as_running() {
    let lavd = SchedExt::Running {
        scheduler: Some("lavd".to_owned()),
    };

    assert!(lavd.is_running("lavd"));
    assert!(!lavd.is_running("bpfland"));
    assert!(!SchedExt::Idle.is_running("lavd"));
    assert!(!SchedExt::Unsupported.is_running("lavd"));
}

#[test]
fn an_unnamed_scheduler_is_never_mistaken_for_the_one_we_wanted() {
    // The kernel says something is attached but will not say what. Treating
    // that as a match would have the step report success over a scheduler
    // somebody else loaded.
    let unknown = SchedExt::Running { scheduler: None };

    assert!(!unknown.is_running("lavd"));
    assert_eq!(unknown.describe(), "an unnamed sched_ext scheduler");
}

#[test]
fn a_versioned_ops_name_still_counts_as_its_scheduler() {
    // The kernel names the attached scheduler with its version and target
    // triple ("lavd_1.1.2_x86_64_unknown_linux_gnu"), so the short name is
    // what anything compares against.
    let versioned = SchedExt::Running {
        scheduler: Some("lavd_1.1.2_x86_64_unknown_linux_gnu".to_owned()),
    };

    assert!(versioned.is_running("lavd"));
    assert!(!versioned.is_running("lavd_x"));
    assert_eq!(versioned.describe(), "lavd");
}

#[test]
fn the_versioned_name_never_reaches_a_journal_or_a_switch_command() {
    // `previous` feeds `scxctl switch -s <name>`, which knows schedulers by
    // the name; a versioned ops string would fail the switch.
    let versioned = SchedExt::Running {
        scheduler: Some("cosmos_1.1.5_x86_64_unknown_linux_gnu".to_owned()),
    };

    assert_eq!(versioned.previous().as_deref(), Some("cosmos"));
}

#[test]
fn a_name_without_a_version_is_returned_whole() {
    let bare = SchedExt::Running {
        scheduler: Some("bpfland".to_owned()),
    };

    assert_eq!(bare.describe(), "bpfland");
    assert_eq!(bare.previous().as_deref(), Some("bpfland"));
}

#[test]
fn only_a_running_scheduler_is_worth_recording_as_the_previous_one() {
    assert_eq!(SchedExt::Unsupported.previous(), None);
    assert_eq!(SchedExt::Idle.previous(), None);
    assert_eq!(
        SchedExt::Running {
            scheduler: Some("bpfland".to_owned())
        }
        .previous()
        .as_deref(),
        Some("bpfland")
    );
}
