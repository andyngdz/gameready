use std::sync::mpsc;

use gameready_core::improvement::ProbeStatus;

use super::*;

fn row(label: &str, status: ProbeStatus) -> Row {
    Row {
        label: label.to_owned(),
        status,
        note: None,
        action: None,
    }
}

fn indicator() -> (Indicator, mpsc::Receiver<Request>) {
    let (requests, incoming) = mpsc::channel();
    (Indicator::new(Ink::Light, requests), incoming)
}

#[test]
fn a_folder_says_how_many_of_its_rows_hold() {
    let rows = [
        row("Swappiness", ProbeStatus::Set),
        row("Split lock", ProbeStatus::Set),
        row("scx lavd", ProbeStatus::Inactive),
    ];

    let MenuItem::SubMenu(folder) = folder("System tunings", &rows) else {
        panic!("expected a submenu");
    };

    assert_eq!(folder.label, "System tunings (2 of 3)");
    assert_eq!(folder.submenu.len(), 3);
}

#[test]
fn a_tuning_carries_a_dot_and_stays_clickable_so_it_highlights() {
    let items = tuning(&row("Swappiness", ProbeStatus::Set));

    assert_eq!(items.len(), 1);
    let MenuItem::Standard(item) = &items[0] else {
        panic!("expected a standard item");
    };

    assert!(item.enabled);
    assert!(!item.icon_data.is_empty());
}

#[test]
fn a_row_with_a_note_gains_a_second_read_only_line_under_it() {
    let mut with_note = row("Proton-GE", ProbeStatus::UpdateAvailable);
    with_note.note = Some("installed GE-Proton11-3, GE-Proton11-5 available".to_owned());

    let items = tuning(&with_note);

    assert_eq!(items.len(), 2);
    let MenuItem::Standard(note_line) = &items[1] else {
        panic!("expected the note as a standard item");
    };
    assert!(!note_line.enabled);
    assert!(note_line.label.contains("GE-Proton11-5"));
}

#[test]
fn a_note_is_read_only_so_it_cannot_be_clicked() {
    let MenuItem::Standard(item) = note("Checking this machine...") else {
        panic!("expected a standard item");
    };

    assert!(!item.enabled);
}

#[test]
fn nothing_holds_in_an_empty_group() {
    assert_eq!(held(&[]), 0);
}

#[test]
fn a_row_whose_action_is_update_proton_ge_asks_for_the_update_when_clicked() {
    let (mut indicator, incoming) = indicator();
    let mut proton = row("Proton-GE", ProbeStatus::UpdateAvailable);
    proton.action = Some(RowAction::UpdateProtonGe);

    let items = tuning(&proton);
    let MenuItem::Standard(item) = &items[0] else {
        panic!("expected a standard item");
    };

    (item.activate)(&mut indicator);

    assert_eq!(incoming.try_recv(), Ok(Request::UpdateProtonGe));
}

#[test]
fn an_update_available_row_without_an_action_stays_read_only() {
    let (mut indicator, incoming) = indicator();
    let proton = row("Proton-GE", ProbeStatus::UpdateAvailable);

    let items = tuning(&proton);
    let MenuItem::Standard(item) = &items[0] else {
        panic!("expected a standard item");
    };

    (item.activate)(&mut indicator);

    assert!(incoming.try_recv().is_err());
}
