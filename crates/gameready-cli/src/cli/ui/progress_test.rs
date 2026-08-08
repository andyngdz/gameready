use gameready_core::improvement::{ImprovementId, OutcomeKind};
use gameready_core::run::{Mode, RunEvent};

use super::{marked_line, result_row, ProgressView};
use crate::cli::ui::layout::{width, Mark};

fn plain(text: &str) -> String {
    console::strip_ansi_codes(text).into_owned()
}

#[test]
fn a_finished_event_clears_the_spinner() {
    let mut view = ProgressView::new();
    view.on_event(RunEvent::Applying {
        step: ImprovementId::from_static("test.step"),
        name: "Test step".to_owned(),
    });
    view.on_event(RunEvent::Finished {
        step: ImprovementId::from_static("test.step"),
        name: "Test step".to_owned(),
        kind: OutcomeKind::Applied,
        detail: Some("verified".to_owned()),
    });
    assert!(!view.region.is_live());
}

#[test]
fn events_without_a_spinner_do_not_panic() {
    let mut view = ProgressView::new();
    view.on_event(RunEvent::Finished {
        step: ImprovementId::from_static("test.step"),
        name: "Already done".to_owned(),
        kind: OutcomeKind::AlreadySet,
        detail: None,
    });
}

#[test]
fn a_step_that_ran_is_named_by_its_short_name_rather_than_the_event_name() {
    // The event carries the step's full title, which is a sentence. The live
    // region is a column of names, and a sentence in it wraps.
    let view = ProgressView::sweeping(Mode::Apply, 0);
    let shown = view.named(
        &ImprovementId::from_static("core.sysctl.max-map-count"),
        "Raise vm.max_map_count for Proton titles",
    );

    assert_eq!(shown, "vm.max_map_count");
}

#[test]
fn a_step_the_catalog_does_not_know_keeps_the_name_the_event_carried() {
    let view = ProgressView::sweeping(Mode::Apply, 0);
    let shown = view.named(&ImprovementId::from_static("test.step"), "Test step");

    assert_eq!(shown, "Test step");
}

#[test]
fn a_sub_phase_reads_against_the_step_it_belongs_to() {
    let mut view = ProgressView::new();
    view.on_event(RunEvent::Applying {
        step: ImprovementId::from_static("test.step"),
        name: "Proton-GE".to_owned(),
    });
    view.on_event(RunEvent::StepProgress {
        step: ImprovementId::from_static("test.step"),
        message: "downloading".to_owned(),
    });

    let message = view.region.saying().expect("a live line");
    assert_eq!(message, "Proton-GE · downloading");
}

#[test]
fn a_download_turns_the_spinner_into_a_bar_and_finishing_takes_it_away() {
    let mut view = ProgressView::new();
    view.on_event(RunEvent::Applying {
        step: ImprovementId::from_static("core.proton.ge"),
        name: "Proton-GE".to_owned(),
    });

    view.on_event(RunEvent::StepBytes {
        step: ImprovementId::from_static("core.proton.ge"),
        done: 65_536,
        total: 186_703_872,
    });
    assert_eq!(view.region.counting(), Some(186_703_872));

    view.on_event(RunEvent::Finished {
        step: ImprovementId::from_static("core.proton.ge"),
        name: "Proton-GE".to_owned(),
        kind: OutcomeKind::Applied,
        detail: Some("installed".to_owned()),
    });
    assert_eq!(view.region.counting(), None);
}

#[test]
fn a_row_runs_its_leader_out_to_the_layout_width() {
    let row = plain(&result_row(
        Mark::Applied,
        "vm.max_map_count",
        "65530 to 2147483642",
    ));

    assert!(row.contains(".."), "{row}");
    assert_eq!(console::measure_text_width(&row), width());
}

#[test]
fn a_step_with_nothing_to_show_for_itself_gets_no_leader() {
    let row = plain(&marked_line(Mark::Applied, "vm.max_map_count"));

    assert_eq!(row, "  ✓ vm.max_map_count");
}
