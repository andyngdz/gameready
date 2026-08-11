use gameready_core::improvement::ProbeStatus;
use gameready_core::infra::exec::MockRunner;
use gameready_core::steps::{core_steps, PROTON_GE_LATEST_URL};
use indoc::indoc;

use super::*;

const OS_RELEASE: &str = indoc! {"
    ID=arch
    NAME=Arch Linux
"};

/// A newer tag than the GE-Proton11-2 the fixture installs, so the sweep reads
/// the machine as "Proton-GE installed, newer available".
const RELEASE_JSON: &str = indoc! {r#"
    {
      "tag_name": "GE-Proton11-5",
      "assets": [
        {
          "name": "GE-Proton11-5.tar.gz",
          "browser_download_url": "https://github.com/dl/GE-Proton11-5.tar.gz"
        },
        {
          "name": "GE-Proton11-5.sha512sum",
          "browser_download_url": "https://github.com/dl/GE-Proton11-5.sha512sum"
        }
      ]
    }
    "#};

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

#[test]
fn an_outdated_proton_ge_is_the_one_clickable_row() {
    let home = std::env::var("HOME").expect("a home to point Steam at");
    let compat = format!("{home}/.steam/root/compatibilitytools.d");
    let runner = arch_machine()
        .with_binary("curl")
        .with_file(format!("{home}/.steam/root"), "")
        .with_file(
            format!("{compat}/GE-Proton11-2/compatibilitytool.vdf"),
            "manifest",
        )
        .answering(format!("curl -sfL {PROTON_GE_LATEST_URL}"), RELEASE_JSON);

    let rows = rows_of(sweep(&runner));

    let proton = rows
        .iter()
        .find(|row| row.label == "Proton-GE")
        .expect("the Proton-GE row exists");
    assert_eq!(proton.status, ProbeStatus::UpdateAvailable);
    assert_eq!(proton.action, Some(RowAction::UpdateProtonGe));
    assert!(
        proton
            .note
            .as_deref()
            .is_some_and(|note| note.contains("GE-Proton11-5")),
        "{proton:?}"
    );

    // Exactly one row in the whole sweep earns a click, and it is the row that
    // just said an update exists. Every other tuning, game rows included, is a
    // menu a user reads, not a button.
    let clickable: Vec<&Row> = rows.iter().filter(|row| row.action.is_some()).collect();
    assert_eq!(clickable.len(), 1, "{rows:?}");
    assert_eq!(clickable[0].label, "Proton-GE");
}
