use super::*;
use crate::infra::exec::MockRunner;

#[test]
fn a_session_carrying_the_group_is_present() {
    let runner = MockRunner::new().answering("id -nG", "tester adm sudo gamemode video\n");
    assert_eq!(
        in_gamemode_group(&runner).expect("reads groups"),
        GamemodeGroup::Present
    );
}

#[test]
fn a_session_without_it_is_absent() {
    let runner = MockRunner::new().answering("id -nG", "tester adm sudo video\n");
    assert_eq!(
        in_gamemode_group(&runner).expect("reads groups"),
        GamemodeGroup::Absent
    );
}

#[test]
fn a_group_name_that_merely_contains_gamemode_does_not_count() {
    // Splitting on whitespace rather than substring matching: `gamemode-admin`
    // is a different group and would not satisfy gamemoded.
    let runner = MockRunner::new().answering("id -nG", "tester gamemode-admin notgamemode\n");
    assert_eq!(
        in_gamemode_group(&runner).expect("reads groups"),
        GamemodeGroup::Absent
    );
}

#[test]
fn a_failing_id_reads_as_absent_rather_than_erroring() {
    // Nothing to restore either way, and refusing the whole run because `id`
    // is missing would be worse than declining one step.
    let runner = MockRunner::new().failing("id -nG");
    assert_eq!(
        in_gamemode_group(&runner).expect("reads groups"),
        GamemodeGroup::Absent
    );
}

#[test]
fn the_command_shown_to_the_user_adds_them_to_the_right_group() {
    assert!(JOIN_GROUP.contains("usermod"), "{JOIN_GROUP}");
    assert!(JOIN_GROUP.contains(GAMEMODE_GROUP), "{JOIN_GROUP}");
}
