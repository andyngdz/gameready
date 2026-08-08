use gameready_core::steps::{core_steps, game_steps};

use super::short_names;

#[test]
fn every_step_the_run_knows_has_a_name_to_show() {
    let names = short_names();

    for step in core_steps().iter().chain(game_steps().iter()) {
        assert!(names.contains_key(&step.id()), "{} is missing", step.id());
    }
}

#[test]
fn the_name_is_the_short_one_rather_than_the_id() {
    let names = short_names();
    let first = core_steps().into_iter().next().expect("a core step");

    assert_eq!(
        names.get(&first.id()).map(String::as_str),
        Some(first.short_name())
    );
}
