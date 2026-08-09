use gameready_core::improvement::ProbeStatus;

use super::*;

fn row(label: &str, status: ProbeStatus) -> Row {
    Row {
        label: label.to_owned(),
        status,
    }
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
    let MenuItem::Standard(item) = tuning(&row("Swappiness", ProbeStatus::Set)) else {
        panic!("expected a standard item");
    };

    assert!(item.enabled);
    assert!(!item.icon_data.is_empty());
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
