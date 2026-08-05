use std::collections::BTreeMap;

use super::*;
use crate::games::domain::{AppId, Wrapper};

fn profile(name: &str, app_id: u32, wrappers: Vec<Wrapper>) -> GameProfile {
    GameProfile {
        name: name.to_owned(),
        app_id: AppId(app_id),
        wrappers,
        env: BTreeMap::new(),
        proton: None,
        override_module: None,
    }
}

#[test]
fn an_empty_catalog_finds_nothing() {
    let catalog = Catalog::new();
    assert!(catalog.is_empty());
    assert!(catalog.find("Deadlock").is_none());
}

#[test]
fn a_user_profile_replaces_the_shipped_one_outright() {
    // Replaced, not merged field by field: a merged profile exists in no file,
    // so the user could not predict the result from what they wrote.
    let mut catalog = Catalog::new();
    catalog.overlay(
        Source::Builtin,
        [profile("Deadlock", 1422450, vec![Wrapper::GameMode])],
    );
    catalog.overlay(Source::User, [profile("Deadlock", 1422450, Vec::new())]);

    let entry = catalog.find("Deadlock").expect("found");
    assert_eq!(entry.source, Source::User);
    assert!(entry.profile.wrappers.is_empty());
    assert_eq!(catalog.len(), 1);
}

#[test]
fn a_system_profile_beats_the_shipped_one_and_loses_to_the_user() {
    let mut catalog = Catalog::new();
    catalog.overlay(Source::Builtin, [profile("Deadlock", 1, Vec::new())]);
    catalog.overlay(Source::System, [profile("Deadlock", 2, Vec::new())]);
    assert_eq!(
        catalog.find("Deadlock").expect("found").source,
        Source::System
    );

    catalog.overlay(Source::User, [profile("Deadlock", 3, Vec::new())]);
    assert_eq!(
        catalog.find("Deadlock").expect("found").source,
        Source::User
    );
}

#[test]
fn a_layer_overrides_by_name_even_when_the_directory_is_spelled_differently() {
    // The three layers are directories people name by hand.
    let mut catalog = Catalog::new();
    catalog.overlay(
        Source::Builtin,
        [profile("Cyberpunk 2077", 1091500, Vec::new())],
    );
    catalog.overlay(
        Source::User,
        [profile("cyberpunk 2077", 1091500, Vec::new())],
    );

    assert_eq!(catalog.len(), 1);
}

#[test]
fn a_lookup_accepts_whatever_casing_the_user_types() {
    let mut catalog = Catalog::new();
    catalog.overlay(
        Source::Builtin,
        [profile("Slay the Spire 2", 2868840, Vec::new())],
    );

    assert!(catalog.find("slay-the-spire-2").is_some());
    assert!(catalog.find("SLAY THE SPIRE 2").is_some());
}

#[test]
fn entries_come_back_in_a_stable_order() {
    // The list is printed, and a list that reshuffles between runs cannot be
    // snapshot tested or read twice.
    let mut catalog = Catalog::new();
    catalog.overlay(
        Source::Builtin,
        [
            profile("Slay the Spire 2", 2868840, Vec::new()),
            profile("Cyberpunk 2077", 1091500, Vec::new()),
            profile("Deadlock", 1422450, Vec::new()),
        ],
    );

    let names: Vec<&str> = catalog
        .entries()
        .iter()
        .map(|entry| entry.profile.name.as_str())
        .collect();
    assert_eq!(names, ["Cyberpunk 2077", "Deadlock", "Slay the Spire 2"]);
}

#[test]
fn every_source_labels_itself() {
    for source in [Source::Builtin, Source::System, Source::User] {
        assert!(!source.label().is_empty());
    }
}
