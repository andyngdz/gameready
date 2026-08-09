use gameready_core::infra::exec::MockRunner;
use gameready_core::steps::core_steps;
use indoc::indoc;

use super::*;

const OS_RELEASE: &str = indoc! {"
    ID=arch
    NAME=Arch Linux
"};

fn arch_machine() -> MockRunner {
    MockRunner::new()
        .answering("uname -r", "6.11.0-generic\n")
        .with_file("/etc/os-release", OS_RELEASE)
}

fn rows_of(snapshot: Snapshot) -> Vec<Row> {
    match snapshot {
        Snapshot::Ready { rows } => rows,
        broken @ Snapshot::Unreadable { .. } => {
            panic!("expected a readable machine, got {broken:?}")
        }
    }
}

#[test]
fn a_machine_that_cannot_be_identified_yields_no_rows_rather_than_wrong_ones() {
    // facts::probe needs uname. Without it the sweep has nothing honest to draw.
    let runner = MockRunner::new().failing("uname -r");

    let snapshot = sweep(&runner);

    assert!(
        matches!(snapshot, Snapshot::Unreadable { .. }),
        "{snapshot:?}"
    );
}

#[test]
fn a_readable_machine_gets_one_row_per_core_step() {
    let rows = rows_of(sweep(&arch_machine()));

    assert_eq!(rows.len(), core_steps().len());
}

#[test]
fn every_row_names_a_step() {
    let rows = rows_of(sweep(&arch_machine()));

    for row in &rows {
        assert!(!row.label.is_empty(), "{row:?}");
    }
}

#[test]
fn rows_stay_in_registry_order_so_the_menu_does_not_reshuffle_between_sweeps() {
    let rows = rows_of(sweep(&arch_machine()));

    let expected: Vec<String> = core_steps()
        .iter()
        .map(|step| step.bar_name().to_owned())
        .collect();
    let actual: Vec<String> = rows.iter().map(|row| row.label.clone()).collect();

    assert_eq!(actual, expected);
}
