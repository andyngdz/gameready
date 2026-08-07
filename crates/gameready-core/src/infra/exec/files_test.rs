use std::os::unix::fs::PermissionsExt as _;

use tempfile::TempDir;

use super::*;

fn mode_of(path: &Path) -> u32 {
    std::fs::metadata(path).expect("stat").permissions().mode() & 0o777
}

#[test]
fn a_privileged_write_creates_the_directories_above_it() {
    // `core.sched.scx-lavd` failed on a real machine with "invalid target
    // '/etc/systemd/system/scx.service.d/10-gameready.conf': No such file or
    // directory", because plain `install` will not make the drop-in directory.
    let install = install_command(
        Path::new("/tmp/gameready-staged-10-gameready.conf"),
        Path::new("/etc/systemd/system/scx.service.d/10-gameready.conf"),
    );

    let rendered = install.to_string();
    assert!(
        rendered.contains(" -D "),
        "without -D a step cannot write into a directory that does not exist yet: {rendered}"
    );
    assert!(rendered.contains(" -m 0644 "), "{rendered}");
    assert!(
        rendered.ends_with("/etc/systemd/system/scx.service.d/10-gameready.conf"),
        "{rendered}"
    );
}

#[test]
fn a_private_file_is_readable_only_by_its_owner() {
    // Steam's config carries an encrypted app ticket and a cloud key, and a
    // backup of it is kept for every run in a directory nothing prunes.
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("secret");

    write_owner_only(&path, "an encrypted ticket").expect("written");

    assert_eq!(mode_of(&path), 0o600, "mode was {:o}", mode_of(&path));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "an encrypted ticket"
    );
}

#[test]
fn a_private_file_is_owner_only_from_the_moment_it_exists() {
    // Creating it readable and tightening it afterwards would leave a window in
    // which anyone on the machine could read the credentials inside, so the
    // mode is set by the open, not by a later chmod.
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("secret");
    write_owner_only(&path, "first").expect("written");
    // Overwriting an existing file must not loosen it either.
    write_owner_only(&path, "second").expect("rewritten");

    assert_eq!(mode_of(&path), 0o600);
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "second");
}

#[test]
fn a_private_file_creates_the_directories_it_needs() {
    // The backups directory for a run may not exist yet when the first step
    // writes a pre-image into it.
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("backups").join("01ABC").join("secret");

    write_owner_only(&path, "x").expect("written");

    assert!(path.is_file());
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
