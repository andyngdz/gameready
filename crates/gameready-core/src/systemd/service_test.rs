use super::*;
use crate::infra::exec::MockRunner;

const UNIT: &str = "tuned.service";

fn systemd_box() -> MockRunner {
    MockRunner::new().with_binary(SYSTEMCTL)
}

#[test]
fn no_systemctl_is_unanswerable_rather_than_absent() {
    // A container has no systemctl. Reporting "unit absent" there would let a
    // conflict check claim the machine is clean when it never looked.
    let runner = MockRunner::new();
    let error = unit_state(&runner, UNIT).expect_err("no systemctl");
    assert!(matches!(error, SystemdError::Unavailable));
}

#[test]
fn a_name_with_no_unit_file_is_absent() {
    // is-enabled prints nothing for a unit that does not exist.
    let runner = systemd_box().answering("systemctl is-enabled tuned.service", "");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::Absent
    );
}

#[test]
fn an_active_unit_is_running() {
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "enabled")
        .answering("systemctl is-active tuned.service", "active");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::Running
    );
}

#[test]
fn a_disabled_unit_that_is_running_still_counts_as_running() {
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "disabled")
        .answering("systemctl is-active tuned.service", "active");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::Running
    );
}

#[test]
fn an_enabled_unit_that_has_not_started_is_pending() {
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "enabled")
        .answering("systemctl is-active tuned.service", "inactive");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::EnabledNotStarted
    );
}

#[test]
fn a_static_unit_counts_as_enabled() {
    // static units have no switch of their own but are pulled in by something
    // that does, so for "will this run" they read the same as enabled.
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "static")
        .answering("systemctl is-active tuned.service", "inactive");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::EnabledNotStarted
    );
}

#[test]
fn a_disabled_and_stopped_unit_is_dormant() {
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "disabled")
        .answering("systemctl is-active tuned.service", "inactive");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::Dormant
    );
}

#[test]
fn a_masked_unit_is_dormant() {
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "masked")
        .answering("systemctl is-active tuned.service", "inactive");
    assert_eq!(
        unit_state(&runner, UNIT).expect("answered"),
        UnitState::Dormant
    );
}

#[test]
fn state_is_read_without_changing_anything() {
    let runner = systemd_box()
        .answering("systemctl is-enabled tuned.service", "enabled")
        .answering("systemctl is-active tuned.service", "active");
    unit_state(&runner, UNIT).expect("answered");
    assert_eq!(
        runner.commands(),
        vec![
            "systemctl is-enabled tuned.service".to_owned(),
            "systemctl is-active tuned.service".to_owned(),
        ]
    );
}
