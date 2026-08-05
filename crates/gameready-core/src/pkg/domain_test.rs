use super::*;

#[test]
fn only_an_available_package_needs_installing() {
    assert!(PackageState::Available.needs_install());
    assert!(!PackageState::Installed { version: None }.needs_install());
    assert!(!PackageState::Unavailable.needs_install());
}

#[test]
fn an_unavailable_package_is_not_obtainable() {
    // A step needing it is not applicable on this system, which is different
    // from the step failing.
    assert!(!PackageState::Unavailable.is_obtainable());
    assert!(PackageState::Available.is_obtainable());
    assert!(PackageState::Installed { version: None }.is_obtainable());
}

#[test]
fn an_outcome_that_installed_nothing_changed_nothing() {
    let outcome = InstallOutcome {
        requested: vec!["gamemode".to_owned()],
        newly_installed: Vec::new(),
    };
    assert!(!outcome.changed_anything());
}
