use std::cell::Cell;

use super::*;

#[test]
fn a_read_only_command_is_never_asked_for_a_password() {
    let asked = Cell::new(0);
    let prompt = || {
        asked.set(asked.get() + 1);
        Ok(())
    };

    Escalation::for_effect(Effect::Reads, &prompt)
        .ask()
        .expect("nothing to ask");

    assert_eq!(asked.get(), 0);
}

#[test]
fn a_mutating_command_asks_once() {
    let asked = Cell::new(0);
    let prompt = || {
        asked.set(asked.get() + 1);
        Ok(())
    };

    Escalation::for_effect(Effect::Mutates, &prompt)
        .ask()
        .expect("the prompt succeeds");

    assert_eq!(asked.get(), 1);
}

#[test]
fn a_refused_password_stops_the_run() {
    let prompt = || Err(anyhow::anyhow!("could not get permission"));

    let refused = Escalation::for_effect(Effect::Mutates, &prompt)
        .ask()
        .expect_err("the prompt failed");

    assert!(
        refused.to_string().contains("could not get permission"),
        "{refused}"
    );
}
