use gameready_core::improvement::{ImprovementId, OutcomeKind};
use gameready_core::run::RunEvent;

use super::ProgressView;

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
    assert!(view.spinner.is_none());
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
