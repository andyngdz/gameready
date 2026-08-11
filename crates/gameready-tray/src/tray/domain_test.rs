use super::*;

fn row(label: &str, status: ProbeStatus) -> Row {
    Row {
        label: label.to_owned(),
        status,
        note: None,
    }
}

#[test]
fn a_tuning_already_in_place_is_its_name_and_nothing_else() {
    // Its dot is green. Restating that in words, or in an ASCII marker beside
    // the dot, says the same thing twice and turns a glance into a read.
    let rendered = row("Shader cache", ProbeStatus::Set).to_string();

    assert_eq!(rendered, "Shader cache");
}

#[test]
fn an_unreadable_machine_never_renders_as_one_with_nothing_to_do() {
    let broken = Snapshot::Unreadable {
        reason: "no uname".to_owned(),
    }
    .to_string();
    let nothing_applicable = Snapshot::Ready { rows: Vec::new() }.to_string();

    assert_ne!(broken, nothing_applicable);
}

#[test]
fn an_unreadable_machine_says_what_failed() {
    let snapshot = Snapshot::Unreadable {
        reason: "no /etc/os-release".to_owned(),
    };

    assert!(snapshot.to_string().contains("no /etc/os-release"));
}

#[test]
fn every_row_of_a_ready_snapshot_gets_its_own_line() {
    let snapshot = Snapshot::Ready {
        rows: vec![
            row("Swappiness", ProbeStatus::Ready),
            row("scx lavd", ProbeStatus::Inactive),
        ],
    };

    assert_eq!(snapshot.to_string().lines().count(), 2);
}
