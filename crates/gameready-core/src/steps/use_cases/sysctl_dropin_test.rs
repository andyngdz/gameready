use super::*;
use crate::infra::exec::MockRunner;
use crate::steps::constants::MANAGED_HEADER;

const KEY: &str = "vm.example";

fn step() -> ImprovementId {
    ImprovementId::from_static("core.sysctl.example")
}

#[test]
fn it_reads_the_value_the_kernel_reports() {
    let runner = MockRunner::new().with_file("/proc/sys/vm/example", "1048576\n");
    let value = read_value(&runner, Path::new("/proc/sys/vm/example"), KEY).expect("reads");
    assert_eq!(value, 1_048_576);
}

#[test]
fn an_unreadable_path_is_an_error_not_a_default() {
    // Falling through to a default would let a step apply without knowing what
    // to put back.
    assert!(read_value(&MockRunner::new(), Path::new("/proc/sys/vm/example"), KEY).is_err());
}

#[test]
fn a_value_that_will_not_parse_is_an_error() {
    let runner = MockRunner::new().with_file("/proc/sys/vm/example", "sometimes\n");
    assert!(read_value(&runner, Path::new("/proc/sys/vm/example"), KEY).is_err());
}

#[test]
fn the_dropin_carries_the_marker_the_step_and_the_run() {
    let run = RunId::generate();
    let body = single_key_dropin(step(), run, KEY, 42);

    assert!(body.starts_with(MANAGED_HEADER), "{body}");
    assert!(body.contains("step=core.sysctl.example"), "{body}");
    assert!(body.contains(&format!("run={run}")), "{body}");
}

#[test]
fn the_dropin_carries_the_assignment_sysctl_reads() {
    let body = single_key_dropin(step(), RunId::generate(), KEY, 42);
    assert!(body.contains("vm.example = 42"), "{body}");
    assert!(body.ends_with('\n'), "{body}");
}

#[test]
fn the_dropin_tells_the_reader_how_to_undo_it() {
    let body = single_key_dropin(step(), RunId::generate(), KEY, 0);
    assert!(body.contains("gameready rollback"), "{body}");
}
