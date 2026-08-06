use super::*;

#[test]
fn a_user_write_lands_the_value_in_the_file() {
    // The root path streams through sudo and is covered by selftest on real
    // hardware; the user path writes directly and is checkable here.
    let dir = tempfile::TempDir::new().expect("temp dir");
    let path = dir.path().join("scheduler");
    let escalator = Escalator::fallback_unprivileged();

    write_sysfs_value(&escalator, &path, "none", Privilege::User).expect("writes");

    assert_eq!(std::fs::read_to_string(&path).expect("read back"), "none");
}
