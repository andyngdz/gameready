use super::*;
use crate::infra::exec::MockRunner;

#[test]
fn a_kernel_missing_a_knob_reports_the_rest_instead_of_failing() {
    // The exact set moves between kernel versions. One absent parameter is no
    // reason to refuse the other four.
    let runner = MockRunner::new()
        .with_file("/proc/sys/vm/compaction_proactiveness", "20\n")
        .with_file("/proc/sys/vm/dirty_ratio", "20\n");

    let readings = survey(&runner).expect("surveys");

    assert_eq!(readings.len(), 2);
    let first = &readings[0];
    assert_eq!(first.knob.key, "vm.compaction_proactiveness");
    assert_eq!(first.current, "20");
}

#[test]
fn a_kernel_with_none_of_them_surveys_empty() {
    let readings = survey(&MockRunner::new()).expect("surveys");
    assert!(readings.is_empty());
}

#[test]
fn already_set_compares_against_the_knobs_own_target() {
    let runner = MockRunner::new().with_file("/proc/sys/vm/compaction_proactiveness", "0\n");
    let readings = survey(&runner).expect("surveys");

    assert!(readings[0].already_set());
}

#[test]
fn every_shipped_knob_is_read_when_the_kernel_has_them_all() {
    let mut runner = MockRunner::new();
    for knob in VM_LATENCY_KNOBS {
        runner = runner.with_file(
            knob.runtime_path().to_string_lossy().as_ref(),
            "999\n".to_owned(),
        );
    }

    let readings = survey(&runner).expect("surveys");

    assert_eq!(readings.len(), VM_LATENCY_KNOBS.len());
    assert!(readings.iter().all(|reading| !reading.already_set()));
}
