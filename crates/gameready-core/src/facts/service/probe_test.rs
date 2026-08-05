use super::*;

#[test]
fn parses_an_ubuntu_release() {
    let version = parse_kernel_release("7.0.0-29-generic").expect("parses");
    assert_eq!(version, KernelVersion::new(7, 0, 0));
}

#[test]
fn parses_an_arch_release() {
    let version = parse_kernel_release("6.14.4-arch1-1").expect("parses");
    assert_eq!(version, KernelVersion::new(6, 14, 4));
}

#[test]
fn parses_a_fedora_release() {
    let version = parse_kernel_release("6.13.8-200.fc41.x86_64").expect("parses");
    assert_eq!(version, KernelVersion::new(6, 13, 8));
}

#[test]
fn treats_a_missing_patch_level_as_zero() {
    // 6.12 and 6.12.0 are the same kernel for a minimum-version check.
    assert_eq!(
        parse_kernel_release("6.12").expect("parses"),
        KernelVersion::new(6, 12, 0)
    );
}

#[test]
fn orders_versions_so_a_minimum_check_works() {
    let running = parse_kernel_release("7.0.0-29-generic").expect("parses");
    let sched_ext_minimum = KernelVersion::new(6, 12, 0);
    assert!(running >= sched_ext_minimum);
}

#[test]
fn rejects_a_release_it_cannot_compare() {
    assert!(parse_kernel_release("not-a-kernel").is_err());
}
