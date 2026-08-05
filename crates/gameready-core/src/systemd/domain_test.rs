use super::*;

#[test]
fn a_running_unit_is_live() {
    assert!(UnitState::Running.is_live());
}

#[test]
fn an_enabled_but_unstarted_unit_is_live() {
    // It acts on the system at the next boot, so warning about it now is the
    // point rather than noise.
    assert!(UnitState::EnabledNotStarted.is_live());
}

#[test]
fn an_installed_but_inert_unit_is_not_live() {
    assert!(!UnitState::Dormant.is_live());
}

#[test]
fn a_missing_unit_is_not_live() {
    assert!(!UnitState::Absent.is_live());
}

#[test]
fn every_state_describes_itself() {
    for state in [
        UnitState::Absent,
        UnitState::Dormant,
        UnitState::EnabledNotStarted,
        UnitState::Running,
    ] {
        assert!(!state.describe().is_empty());
    }
}
