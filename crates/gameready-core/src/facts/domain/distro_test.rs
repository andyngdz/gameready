use super::*;

#[test]
fn each_family_names_its_own_package_manager() {
    assert_eq!(Family::Arch.package_manager(), PackageManagerKind::Pacman);
    assert_eq!(Family::Debian.package_manager(), PackageManagerKind::Apt);
    assert_eq!(Family::Fedora.package_manager(), PackageManagerKind::Dnf);
}

#[test]
fn the_debian_family_drives_apt_get_not_apt() {
    // `apt` is the interactive front end and warns that it has no stable CLI.
    // Scripting it is what breaks between releases.
    assert_eq!(PackageManagerKind::Apt.binary(), "apt-get");
}
