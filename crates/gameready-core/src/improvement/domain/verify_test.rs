use super::*;

#[test]
fn an_empty_verification_does_not_pass() {
    // A step that proved nothing must not be reported as applied.
    assert!(!Verification::new().passed());
}

#[test]
fn a_matching_readback_passes() {
    let verification = Verification::new().check(Check::equals(
        "vm.max_map_count",
        "2147483642",
        "2147483642",
    ));
    assert!(verification.passed());
    assert_eq!(verification.failed_count(), 0);
}

#[test]
fn one_failing_check_fails_the_whole_verification() {
    let verification = Verification::new()
        .check(Check::equals("runtime value", "2147483642", "2147483642"))
        .check(Check::equals("file exists", "yes", "no"));
    assert!(!verification.passed());
    assert_eq!(verification.failed_count(), 1);
    assert_eq!(verification.total_count(), 2);
}
