use std::path::{Path, PathBuf};

use gameready_core::improvement::ImprovementId;
use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::{PriorUnitState, RunId, Undo};
use gameready_core::rollback::{PlannedUndo, RollbackPlan, UndoOutcome, UndoReport};
use gameready_core::steam::{PriorBlock, PriorScalar, PriorSection};
use gameready_core::steps::SteamProton;
use indoc::indoc;

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
                Undo::RestoreUnit {
                    unit: "tuned.service".to_owned(),
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
                        "lutris".to_owned(),
                    ],
                },
                UndoOutcome::Left {
                    reason: "left installed".to_owned(),
                },
            ),
        ],
    };

    let text = RollbackSummary::new(&report, Path::new("/state/journal.jsonl")).to_string();

    // Each subject and its note read as one row, whatever the column ends up
    // being: the layout is the table's job, and asserting on the spacing here
    // would only pin how wide the widest subject happened to be.
    let row = |subject: &str| {
        text.lines()
            .find(|line| line.contains(subject))
            .unwrap_or_else(|| panic!("no row for {subject} in {text}"))
            .to_owned()
    };
    assert!(row("vm.max_map_count").contains("back to 65530"), "{text}");
    assert!(
        row("I/O scheduler nvme0n1").contains("back to mq-deadline"),
        "{text}"
    );
    assert!(
        row("tuned.service").contains("could not stop: unit not found"),
        "{text}"
    );
    // The package report is a note, not a row, and reads as a list.
    assert!(
        text.contains("gamemode, mangohud and lutris are still installed"),
        "{text}"
    );
    assert!(text.contains("Remove them yourself"), "{text}");
    assert!(text.contains("2 reverted, 1 failed"), "{text}");
}

#[test]
fn preview_reads_the_current_value_beside_what_it_will_restore_to() {
    let runner = MockRunner::new().with_file("/proc/sys/vm/max_map_count", "2147483642\n");
    let plan = RollbackPlan {
        run: RunId::generate(),
        undos: vec![PlannedUndo {
            step: ImprovementId::from_static("core.sysctl.max-map-count"),
            seq: 0,
            undo: Undo::SetSysctl {
                key: "vm.max_map_count".to_owned(),
                value: "1048576".to_owned(),
            },
        }],
    };

    let text = preview(&plan, &runner);

    assert!(text.contains("vm.max_map_count"), "{text}");
    assert!(text.contains("2147483642 → 1048576"), "{text}");
}

#[test]
fn preview_names_a_steam_block_and_its_current_value() {
    let runner = MockRunner::new().with_file(
        "/steam/config/config.vdf",
        indoc! {r#"
            "InstallConfigStore"
            {
                "Software"
                {
                    "Valve"
                    {
                        "Steam"
                        {
                            "CompatToolMapping"
                            {
                                "1422450"
                                {
                                    "name"        "GE-Proton11-5-x86_64"
                                }
                            }
                        }
                    }
                }
            }
        "#},
    );
    let plan = RollbackPlan {
        run: RunId::generate(),
        undos: vec![PlannedUndo {
            step: SteamProton::id_const(),
            seq: 0,
            undo: Undo::RestoreSteamConfig {
                path: "/steam/config/config.vdf".into(),
                sections: vec![PriorSection {
                    section: [
                        "InstallConfigStore",
                        "Software",
                        "Valve",
                        "Steam",
                        "CompatToolMapping",
                        "1422450",
                    ]
                    .iter()
                    .map(|part| (*part).to_owned())
                    .collect(),
                    prior: PriorBlock::Present {
                        entries: vec![PriorScalar {
                            key: "name".to_owned(),
                            value: Some("proton_experimental".to_owned()),
                        }],
                    },
                }],
            },
        }],
    };

    let text = preview(&plan, &runner);

    assert!(text.contains("Proton pin · 1422450"), "{text}");
    assert!(
        text.contains("GE-Proton11-5-x86_64 → proton_experimental"),
        "{text}"
    );
}

#[test]
fn preview_that_cannot_read_a_value_shows_only_the_target() {
    let runner = MockRunner::new();
    let plan = RollbackPlan {
        run: RunId::generate(),
        undos: vec![PlannedUndo {
            step: ImprovementId::from_static("core.sysctl.max-map-count"),
            seq: 0,
            undo: Undo::SetSysctl {
                key: "vm.max_map_count".to_owned(),
                value: "1048576".to_owned(),
            },
        }],
    };

    let text = preview(&plan, &runner);

    assert!(text.contains("vm.max_map_count"), "{text}");
    assert!(text.contains("→ 1048576"), "{text}");
    assert!(!text.contains("2147483642"), "{text}");
}
