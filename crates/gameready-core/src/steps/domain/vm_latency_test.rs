use super::*;

#[test]
fn a_runtime_path_swaps_only_the_dots() {
    let knob = LatencyKnob {
        key: "vm.page-lock-unfairness",
        target: "1",
        why: "unused here",
    };

    assert_eq!(
        knob.runtime_path(),
        std::path::Path::new("/proc/sys/vm/page-lock-unfairness")
    );
}

#[test]
fn every_shipped_knob_resolves_under_proc_sys() {
    for knob in VM_LATENCY_KNOBS {
        let path = knob.runtime_path();
        assert!(path.starts_with("/proc/sys"), "{path:?}");
        assert!(!knob.target.is_empty(), "{}", knob.key);
        assert!(!knob.why.is_empty(), "{}", knob.key);
    }
}

#[test]
fn swappiness_stays_out_of_this_step() {
    // core.memory.swappiness owns vm.swappiness. Two steps writing one key
    // would clobber each other's rollback.
    assert!(
        !VM_LATENCY_KNOBS
            .iter()
            .any(|knob| knob.key == "vm.swappiness"),
        "vm.swappiness belongs to core.memory.swappiness"
    );
}
