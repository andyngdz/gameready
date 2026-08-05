use indoc::indoc;

use super::*;
use crate::facts::PackageManagerKind;

/// Real `/etc/os-release` shapes. The values matter more than they look: the
/// upstream project this replaces matched `ID_LIKE` as a whole string, which
/// silently fails on Pop!_OS, where it is a two-word list.
fn ubuntu() -> &'static str {
    indoc! {r#"
        PRETTY_NAME="Ubuntu 26.04 LTS"
        NAME="Ubuntu"
        VERSION_ID="26.04"
        ID=ubuntu
        ID_LIKE=debian
    "#}
}

fn pop_os() -> &'static str {
    indoc! {r#"
        PRETTY_NAME="Pop!_OS 22.04 LTS"
        NAME="Pop!_OS"
        VERSION_ID="22.04"
        ID=pop
        ID_LIKE="ubuntu debian"
    "#}
}

fn arch() -> &'static str {
    indoc! {r#"
        NAME="Arch Linux"
        PRETTY_NAME="Arch Linux"
        ID=arch
        BUILD_ID=rolling
    "#}
}

fn nobara() -> &'static str {
    indoc! {r#"
        NAME="Nobara Linux"
        VERSION_ID="41"
        ID=nobara
        ID_LIKE="fedora"
        PRETTY_NAME="Nobara Linux 41"
    "#}
}

fn bazzite() -> &'static str {
    indoc! {r#"
        NAME="Bazzite"
        ID=bazzite
        ID_LIKE="fedora"
        VERSION_ID="41"
        VARIANT_ID=bazzite-deck
        PRETTY_NAME="Bazzite 41"
    "#}
}

#[test]
fn ubuntu_is_the_debian_family() {
    let distro = parse(ubuntu()).expect("parses");
    assert_eq!(distro.id, "ubuntu");
    assert_eq!(distro.family, Family::Debian);
    assert_eq!(distro.version_id.as_deref(), Some("26.04"));
    assert_eq!(distro.package_manager(), PackageManagerKind::Apt);
}

#[test]
fn pop_os_resolves_through_a_multi_word_id_like() {
    // ID_LIKE is a space-separated list, so matching the whole string fails
    // here. That exact bug is why this fixture exists.
    let distro = parse(pop_os()).expect("parses");
    assert_eq!(distro.id, "pop");
    assert_eq!(distro.family, Family::Debian);
}

#[test]
fn a_rolling_release_has_no_version() {
    let distro = parse(arch()).expect("parses");
    assert_eq!(distro.family, Family::Arch);
    assert_eq!(distro.version_id, None);
}

#[test]
fn a_derivative_that_names_itself_is_matched_directly() {
    let distro = parse(nobara()).expect("parses");
    assert_eq!(distro.id, "nobara");
    assert_eq!(distro.family, Family::Fedora);
}

#[test]
fn an_image_based_system_is_flagged_as_such() {
    // Writing outside /etc is the wrong operation here, not merely a
    // permission error, so steps have to be able to tell.
    let distro = parse(bazzite()).expect("parses");
    assert_eq!(distro.family, Family::Fedora);
    assert_eq!(distro.root_filesystem, RootFilesystem::ImageBased);
}

#[test]
fn a_normal_system_is_not_flagged_as_image_based() {
    assert_eq!(
        parse(ubuntu()).expect("parses").root_filesystem,
        RootFilesystem::Mutable
    );
}

#[test]
fn quotes_are_stripped_from_values() {
    let distro = parse(ubuntu()).expect("parses");
    assert_eq!(distro.name, "Ubuntu 26.04 LTS");
}

#[test]
fn comments_are_ignored() {
    let contents = indoc! {r#"
        # written by the vendor
        ID=arch
    "#};
    assert_eq!(parse(contents).expect("parses").family, Family::Arch);
}

#[test]
fn a_file_without_an_id_is_rejected() {
    let error = parse("PRETTY_NAME=\"Something\"\n").expect_err("no ID");
    assert!(matches!(error, FactsError::OsRelease { missing: "ID" }));
}

#[test]
fn an_unsupported_distro_says_what_it_claimed_to_be() {
    let contents = indoc! {r#"
        ID=gentoo
        ID_LIKE=""
    "#};
    let error = parse(contents).expect_err("unsupported");
    let message = error.to_string();
    assert!(message.contains("gentoo"), "{message}");
}

#[test]
fn every_supported_family_is_reachable_from_a_real_id() {
    for (contents, expected) in [
        (arch(), Family::Arch),
        (ubuntu(), Family::Debian),
        (nobara(), Family::Fedora),
    ] {
        assert_eq!(parse(contents).expect("parses").family, expected);
    }
}
