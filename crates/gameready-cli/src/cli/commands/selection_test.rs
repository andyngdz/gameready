use super::*;

#[test]
fn no_step_selects_every_step() {
    let all = select_steps(None).expect("selects");
    assert_eq!(all.len(), core_steps().len());
}

#[test]
fn a_named_step_selects_only_that_one() {
    let selected = select_steps(Some("core.io.scheduler")).expect("selects");
    assert_eq!(selected.len(), 1);
    let only = &selected[0];
    assert_eq!(only.id(), ImprovementId::from_static("core.io.scheduler"));
}

#[test]
fn an_unknown_but_well_formed_step_id_is_an_error() {
    assert!(select_steps(Some("core.does.not.exist")).is_err());
}

#[test]
fn a_malformed_step_id_is_an_error() {
    assert!(select_steps(Some("Not A Valid Id")).is_err());
}
