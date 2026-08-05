use std::path::Path;

use gameready_core::improvement::ImprovementId;
use gameready_core::journal::{RunId, Undo};
use gameready_core::rollback::{UndoOutcome, UndoReport};

use super::*;

fn report(outcome: UndoOutcome) -> RollbackReport {
    RollbackReport {
        run: RunId::generate(),
        undos: vec![UndoReport {
            step: ImprovementId::from_static("core.sysctl.max-map-count"),
            undo: Undo::SetSysctl {
                key: "vm.max_map_count".to_owned(),
                value: "1048576".to_owned(),
            },
            outcome,
        }],
    }
}

#[test]
fn a_reverted_change_is_counted_and_described() {
    let outcome = UndoOutcome::Reverted {
        detail: "vm.max_map_count back to 1048576".to_owned(),
    };
    let report = report(outcome);
    let text = RollbackSummary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    assert!(text.contains("ok vm.max_map_count back to 1048576"));
    assert!(text.contains("1 reverted, 0 failed"));
}

#[test]
fn a_refused_change_says_why_and_is_not_counted_as_failed() {
    // Leaving an edited file alone is the designed behaviour, not an error.
    let outcome = UndoOutcome::Refused {
        reason: "changed since gameready wrote it".to_owned(),
    };
    let report = report(outcome);
    let text = RollbackSummary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    assert!(text.contains("changed since gameready wrote it"));
    assert!(text.contains("0 failed"));
}
