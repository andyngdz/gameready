use super::*;
use crate::infra::exec::MockRunner;

#[test]
fn a_kernel_without_the_state_file_has_no_sched_ext() {
    // Not an error to report. A kernel built without sched_ext simply has no
    // such file, and every caller would turn the read failure into this answer.
    let runner = MockRunner::new();

    assert_eq!(read_sched_ext(&runner), SchedExt::Unsupported);
}

#[test]
fn a_disabled_kernel_is_idle_and_ready_for_a_scheduler() {
    // The exact state this machine is in: kernel 7.0.0 with sched_ext built in
    // and nothing attached.
    let runner = MockRunner::new().with_file(SCHED_EXT_STATE, "disabled\n");

    assert_eq!(read_sched_ext(&runner), SchedExt::Idle);
}

#[test]
fn an_enabled_kernel_reports_the_scheduler_it_named() {
    let runner = MockRunner::new()
        .with_file(SCHED_EXT_STATE, "enabled\n")
        .with_file(SCHED_EXT_OPS, "lavd\n");

    assert_eq!(
        read_sched_ext(&runner),
        SchedExt::Running {
            scheduler: Some("lavd".to_owned())
        }
    );
}

#[test]
fn an_enabled_kernel_that_names_nothing_still_reports_something_attached() {
    // `root/ops` only exists while a scheduler is attached. A kernel that says
    // enabled without exposing the name is one gameready does not understand,
    // and the safe reading is that somebody else owns the scheduler.
    let runner = MockRunner::new().with_file(SCHED_EXT_STATE, "enabled\n");

    assert_eq!(
        read_sched_ext(&runner),
        SchedExt::Running { scheduler: None }
    );
}
