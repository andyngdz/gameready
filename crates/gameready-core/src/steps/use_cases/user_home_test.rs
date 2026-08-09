use super::*;

#[test]
fn it_reads_the_home_the_environment_reports() {
    // Reading the live variable rather than setting one: std::env::set_var is
    // process-global, so a test that changed it would race every other test in
    // the binary.
    let expected = std::env::var(HOME_VAR).map(PathBuf::from);
    if let Ok(expected) = expected {
        assert_eq!(user_home(), expected);
    } else {
        assert_eq!(user_home(), PathBuf::from(HOMELESS_FALLBACK));
    }
}

#[test]
fn the_result_is_always_absolute() {
    // A relative path here would put a per-user file wherever the process
    // happened to start, which is the failure this fallback exists to prevent.
    assert!(user_home().is_absolute(), "{:?}", user_home());
}
