use std::time::Duration;

use gameready_core::improvement::{ImprovementId, Outcome, Verification};
use gameready_core::journal::RunId;
use gameready_core::run::{Mode, RunReport, StepReport};

use super::LaunchReport;

fn report(outcome: Outcome) -> RunReport {
    RunReport {
        run: RunId::generate(),
        mode: Mode::Apply,
        steps: vec![StepReport {
            step: ImprovementId::from_static("game.steam.launch-options"),
            name: "Set Steam launch options".to_owned(),
            outcome,
        }],
        installed_dependencies: Vec::new(),
        took: Duration::from_millis(1),
    }
}

#[test]
fn a_successful_write_says_steam_is_restarting() {
    let rendered = LaunchReport::new(&report(Outcome::Applied {
        changes: Vec::new(),
        verification: Verification::new(),
        took: Duration::from_millis(1),
    }))
    .to_string();

    assert!(rendered.contains("Launch options set"), "{rendered}");
    assert!(rendered.contains("restarting"), "{rendered}");
}

#[test]
fn already_set_is_reported_quietly() {
    let rendered = LaunchReport::new(&report(Outcome::AlreadyApplied {
        evidence: "already set".to_owned(),
    }))
    .to_string();

    assert!(rendered.contains("already set"), "{rendered}");
}

#[test]
fn a_failure_carries_its_detail() {
    let rendered = LaunchReport::new(&report(Outcome::Failed {
        error: "the config could not be parsed".to_owned(),
        rolled_back: gameready_core::improvement::RollbackStatus::Succeeded,
    }))
    .to_string();

    assert!(rendered.contains("could not be parsed"), "{rendered}");
}
