use std::path::{Path, PathBuf};

use gameready_core::improvement::ImprovementId;
use gameready_core::journal::{PriorUnitState, RunId, Undo};
use gameready_core::rollback::{UndoOutcome, UndoReport};

use super::*;

fn entry(undo: Undo, outcome: UndoOutcome) -> UndoReport {
    UndoReport {
        step: ImprovementId::from_static("core.test"),
        undo,
        outcome,
    }
}

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

    assert!(text.contains("vm.max_map_count"), "{text}");
    assert!(text.contains("back to 1048576"), "{text}");
    // Nothing failed, so nothing says so: a zero the reader has to find is a
    // question they were made to ask.
    assert!(text.contains("1 reverted"), "{text}");
    assert!(!text.contains("failed"), "{text}");
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
    assert!(!text.contains("failed"), "{text}");
}

#[test]
fn each_undo_reads_as_a_subject_and_what_became_of_it() {
    let report = RollbackReport {
        run: RunId::generate(),
        undos: vec![
            entry(
                Undo::SetSysctl {
                    key: "vm.max_map_count".to_owned(),
                    value: "65530".to_owned(),
                },
                UndoOutcome::Reverted {
                    detail: "vm.max_map_count back to 65530".to_owned(),
                },
            ),
            entry(
                Undo::WriteSysfs {
                    path: PathBuf::from("/sys/block/nvme0n1/queue/scheduler"),
                    value: "mq-deadline".to_owned(),
                },
                UndoOutcome::Reverted {
                    detail: "restored".to_owned(),
                },
            ),
            entry(
                Undo::RestoreScxScheduler { previous: None },
                UndoOutcome::Reverted {
                    detail: "restored".to_owned(),
                },
            ),
            entry(
                Undo::RestoreUnit {
                    unit: "scx_loader.service".to_owned(),
                    prior: PriorUnitState::WasDisabled,
                },
                UndoOutcome::Failed {
                    error: "could not stop: unit not found".to_owned(),
                },
            ),
            entry(
                Undo::ReportPackages {
                    manager: "apt".to_owned(),
                    installed: vec![
                        "gamemode".to_owned(),
                        "mangohud".to_owned(),
                        "scx-scheds".to_owned(),
                    ],
                },
                UndoOutcome::Left {
                    reason: "left installed".to_owned(),
                },
            ),
        ],
    };

    let text = RollbackSummary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    // Subject and note, split by the middle dot.
    assert!(
        text.contains("vm.max_map_count \u{b7} back to 65530"),
        "{text}"
    );
    assert!(
        text.contains("I/O scheduler nvme0n1 \u{b7} back to mq-deadline"),
        "{text}"
    );
    assert!(
        text.contains("CPU scheduler \u{b7} sched_ext unloaded, kernel scheduler back"),
        "{text}"
    );
    assert!(
        text.contains("scx_loader.service \u{b7} could not stop: unit not found"),
        "{text}"
    );
    // The package report is a note, not a row, and reads as a list.
    assert!(
        text.contains("gamemode, mangohud and scx-scheds are still installed"),
        "{text}"
    );
    assert!(text.contains("--purge-packages"), "{text}");
    assert!(text.contains("3 reverted, 1 failed"), "{text}");
}
