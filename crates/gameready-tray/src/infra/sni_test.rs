use std::sync::mpsc;

use gameready_core::games::AppId;
use gameready_core::improvement::ProbeStatus;

use crate::tray::Row;

use super::*;

fn row(label: &str, status: ProbeStatus) -> Row {
    Row {
        label: label.to_owned(),
        status,
        note: None,
    }
}

fn playing(rows: Vec<Row>) -> Activity {
    Activity::Playing {
        game: "Deadlock".to_owned(),
        app_id: AppId(1_422_450),
        rows,
    }
}

fn indicator() -> (Indicator, mpsc::Receiver<Request>) {
    let (requests, incoming) = mpsc::channel();
    (Indicator::new(Ink::Light, requests), incoming)
}

fn label_of(item: &MenuItem<Indicator>) -> String {
    match item {
        MenuItem::Standard(standard) => standard.label.clone(),
        MenuItem::SubMenu(sub) => sub.label.clone(),
        MenuItem::Separator => "---".to_owned(),
        MenuItem::Checkmark(_) | MenuItem::RadioGroup(_) => String::new(),
    }
}

fn labels(indicator: &Indicator) -> Vec<String> {
    indicator.menu().iter().map(label_of).collect()
}

/// The labels inside the submenu at `index` of the top level.
fn inside(indicator: &Indicator, index: usize) -> Vec<String> {
    match &indicator.menu()[index] {
        MenuItem::SubMenu(sub) => sub.submenu.iter().map(label_of).collect(),
        other @ (MenuItem::Standard(_)
        | MenuItem::Separator
        | MenuItem::Checkmark(_)
        | MenuItem::RadioGroup(_)) => {
            panic!("expected a submenu, got {:?}", label_of(other))
        }
    }
}

#[test]
fn a_tray_that_has_not_read_the_machine_says_so_rather_than_showing_nothing() {
    let (indicator, _incoming) = indicator();

    let labels = labels(&indicator);

    assert_eq!(labels.first().map(String::as_str), Some(CHECKING));
    assert!(indicator.tool_tip().description.contains("Checking"));
}

fn two_tunings() -> Snapshot {
    Snapshot::Ready {
        rows: vec![
            row("Swappiness", ProbeStatus::Set),
            row("scx lavd", ProbeStatus::Inactive),
        ],
    }
}

#[test]
fn the_top_level_is_one_line_per_subject_with_the_actions_last() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(two_tunings());

    let labels = labels(&indicator);

    // Thirteen tunings at the top level is a menu nobody reads.
    assert_eq!(labels.len(), 4, "{labels:?}");
    assert_eq!(labels[1], "---");
    assert_eq!(labels[2], REFRESH);
    assert_eq!(labels[3], QUIT);
}

#[test]
fn the_system_line_says_how_many_hold_so_the_menu_need_not_be_opened() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(two_tunings());

    assert_eq!(labels(&indicator)[0], format!("{SYSTEM} (1 of 2)"));
}

#[test]
fn every_tuning_lives_one_hover_away() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(two_tunings());

    let rows = inside(&indicator, 0);

    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(rows[0].contains("Swappiness"), "{rows:?}");
    assert!(rows[1].contains("scx lavd"), "{rows:?}");
}

#[test]
fn a_tuning_row_lights_up_on_hover_and_still_changes_nothing() {
    let (mut indicator, incoming) = indicator();
    indicator.show(Snapshot::Ready {
        rows: vec![row("Swappiness", ProbeStatus::Set)],
    });

    let MenuItem::SubMenu(system) = indicator.menu().remove(0) else {
        panic!("expected the system submenu");
    };
    let mut inner = system.submenu;
    let MenuItem::Standard(first) = inner.remove(0) else {
        panic!("expected a standard item");
    };

    // Disabled rows are drawn greyed and never highlight, which reads as a
    // broken menu beside every other tray on the bar.
    assert!(first.enabled);
    assert!(!first.icon_data.is_empty(), "the row lost its dot");

    // Read-only is kept by the handler doing nothing, not by refusing to light
    // up: clicking a tuning must not ask the main loop for anything.
    (first.activate)(&mut indicator);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn a_machine_that_could_not_be_read_says_why_instead_of_listing_nothing() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(Snapshot::Unreadable {
        reason: "no /etc/os-release".to_owned(),
    });

    let labels = labels(&indicator);

    assert_eq!(
        labels.first().map(String::as_str),
        Some("no /etc/os-release")
    );
    assert!(indicator
        .tool_tip()
        .description
        .contains("no /etc/os-release"));
}

#[test]
fn the_hover_line_counts_the_tunings_that_hold() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(Snapshot::Ready {
        rows: vec![
            row("Swappiness", ProbeStatus::Set),
            row("split lock", ProbeStatus::Set),
            row("scx lavd", ProbeStatus::Inactive),
        ],
    });

    assert_eq!(indicator.tool_tip().description, "2 of 3 tunings in place");
}

#[test]
fn the_icon_goes_green_while_a_configured_game_runs_and_back_when_it_stops() {
    let pixels = |indicator: &Indicator| -> Vec<Vec<u8>> {
        indicator
            .icon_pixmap()
            .into_iter()
            .map(|icon| icon.data)
            .collect()
    };

    let (mut indicator, _incoming) = indicator();
    let resting = pixels(&indicator);

    indicator.playing(playing(Vec::new()));
    let live = pixels(&indicator);

    indicator.playing(Activity::Idle);

    assert_ne!(resting, live);
    assert_eq!(resting, pixels(&indicator));
}

#[test]
fn a_running_game_is_named_in_the_menu_rather_than_only_in_the_icon() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(Snapshot::Ready {
        rows: vec![row("Swappiness", ProbeStatus::Set)],
    });
    indicator.playing(playing(Vec::new()));

    let labels = labels(&indicator);

    assert!(
        labels.iter().any(|label| label.contains("Deadlock")),
        "{labels:?}"
    );
}

#[test]
fn a_running_game_gets_its_own_submenu_holding_only_its_own_tunings() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(two_tunings());
    indicator.playing(playing(vec![
        row("Launch options", ProbeStatus::Set),
        row("Proton pin", ProbeStatus::Ready),
    ]));

    let labels = labels(&indicator);

    assert_eq!(labels[0], format!("{SYSTEM} (1 of 2)"));
    assert_eq!(labels[1], "Deadlock (1 of 2)");
    let game = inside(&indicator, 1);
    assert!(game[0].contains("Launch options"), "{game:?}");
    assert!(game[1].contains("Proton pin"), "{game:?}");
}

#[test]
fn a_game_with_nothing_to_show_is_named_rather_than_given_an_empty_submenu() {
    let (mut indicator, _incoming) = indicator();
    indicator.show(two_tunings());
    indicator.playing(playing(Vec::new()));

    let labels = labels(&indicator);

    // A submenu the user opens to find nothing is a promise the row could not
    // keep. Steam may not be installed, or the game may have no profile.
    assert!(!labels[1].contains("Deadlock ("), "{labels:?}");
    assert!(
        labels.iter().any(|label| label == "Deadlock is running"),
        "{labels:?}"
    );
}

#[test]
fn clicking_refresh_posts_a_request_rather_than_sweeping_on_the_menu_thread() {
    let (mut indicator, incoming) = indicator();
    let mut menu = indicator.menu();
    let MenuItem::Standard(refresh) = menu.remove(menu.len() - 2) else {
        panic!("expected a standard item");
    };

    (refresh.activate)(&mut indicator);

    assert_eq!(incoming.try_recv(), Ok(Request::Refresh));
}

#[test]
fn clicking_quit_asks_the_main_loop_to_stop() {
    let (mut indicator, incoming) = indicator();
    let mut menu = indicator.menu();
    let MenuItem::Standard(quit) = menu.remove(menu.len() - 1) else {
        panic!("expected a standard item");
    };

    (quit.activate)(&mut indicator);

    assert_eq!(incoming.try_recv(), Ok(Request::Quit));
}

#[test]
fn a_click_after_the_main_loop_has_gone_is_dropped_rather_than_panicking() {
    let (requests, incoming) = mpsc::channel();
    let indicator = Indicator::new(Ink::Light, requests);
    drop(incoming);

    indicator.ask(Request::Refresh);
}
