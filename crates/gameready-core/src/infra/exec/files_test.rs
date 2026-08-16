use tempfile::TempDir;

use super::*;

#[test]
fn a_privileged_write_creates_the_directories_above_it() {
    // A managed file can land in a directory that does not exist yet, and plain
    // `install` will not make it. The -D flag is what does.
    let install = install_command(
        Path::new("/tmp/gameready-staged-10-gameready.conf"),
        Path::new("/etc/foo.d/10-gameready.conf"),
    );

    let rendered = install.to_string();
    assert!(
        rendered.contains(" -D "),
        "without -D a step cannot write into a directory that does not exist yet: {rendered}"
    );
    assert!(rendered.contains(" -m 0644 "), "{rendered}");
    assert!(
        rendered.ends_with("/etc/foo.d/10-gameready.conf"),
        "{rendered}"
    );
}

#[test]
fn a_staged_file_carries_the_contents_it_was_given() {
    let dir = TempDir::new().expect("temp dir");
    let staged = stage_temp_file(
        &dir.path().join("99-gameready.conf"),
        "vm.max_map_count = 1",
    )
    .expect("staged");

    assert_eq!(
        std::fs::read_to_string(&staged).expect("read"),
        "vm.max_map_count = 1"
    );
    std::fs::remove_file(&staged).expect("clean up");
}
