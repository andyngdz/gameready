use super::*;
use crate::steps::use_cases::MaxMapCount;

#[test]
fn every_shipped_step_has_a_well_formed_id() {
    // Ids are journal keys. A malformed one authored as a literal is only
    // caught here, because from_static cannot validate at compile time.
    for step in core_steps() {
        let id = step.id();
        assert!(
            ImprovementId::parse(id.as_str()).is_ok(),
            "{id} is not a usable journal key"
        );
    }
}

#[test]
fn every_shipped_step_explains_itself() {
    for step in core_steps() {
        let id = step.id();
        assert!(!step.name().is_empty(), "{id} has no name");
        assert!(
            step.rationale().len() > 40,
            "{id} has no rationale a user could act on"
        );
    }
}

#[test]
fn step_ids_are_unique() {
    let mut ids: Vec<_> = core_steps().iter().map(|step| step.id()).collect();
    let before = ids.len();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), before, "two steps share a journal key");
}

#[test]
fn a_step_can_be_found_by_id() {
    let found = find_core_step(&MaxMapCount::id_const()).expect("registered");
    assert_eq!(found.id(), MaxMapCount::id_const());
}

#[test]
fn an_unknown_id_finds_nothing() {
    let unknown = ImprovementId::from_static("core.does.not-exist");
    assert!(find_core_step(&unknown).is_none());
}
