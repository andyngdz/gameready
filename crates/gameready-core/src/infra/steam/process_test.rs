use super::*;
use crate::infra::exec::MockRunner;

#[test]
fn a_machine_where_pgrep_finds_steam_reports_it_running() {
    // Unseeded commands succeed in the mock, which is pgrep's "found it".
    let runner = MockRunner::new();
    assert!(is_running(&runner));
}

#[test]
fn a_machine_where_pgrep_finds_nothing_reports_it_stopped() {
    let runner = MockRunner::new().failing("pgrep -x steam");
    assert!(!is_running(&runner));
}

#[test]
fn a_steam_that_is_already_stopped_is_not_asked_to_quit() {
    let runner = MockRunner::new().failing("pgrep -x steam");
    shutdown(&runner).expect("nothing to do");

    assert!(
        !runner
            .commands()
            .iter()
            .any(|command| command.contains("-shutdown")),
        "{:?}",
        runner.commands()
    );
}
