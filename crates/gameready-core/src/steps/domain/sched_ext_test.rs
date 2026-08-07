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
