use indoc::indoc;
use tempfile::TempDir;

use super::*;
use crate::facts::{Family, SystemFacts};
use crate::improvement::ImprovementId;
use crate::infra::exec::MockRunner;
use crate::journal::{Journal, RunId, StatePaths};
use crate::steam::{PriorBlock, PriorScalar, PriorSection};

const CONFIG: &str = "/steam/userdata/1/config/localconfig.vdf";

fn written() -> String {
    indoc! {r#"
        "UserLocalConfigStore"
        {
            "Software"
            {
                "Valve"
                {
                    "Steam"
                    {
                        "apps"
                        {
                            "1422450"
                            {
                                "LaunchOptions"		"gamemoderun %command%"
                            }
                        }
                    }
                }
            }
        }
    "#}
    .to_owned()
}

fn change(value: Option<&str>) -> Change {
    Change::SteamConfigWritten {
        path: CONFIG.into(),
        sections: vec![PriorSection {
            section: [
                "UserLocalConfigStore",
                "Software",
                "Valve",
                "Steam",
                "apps",
                "1422450",
            ]
            .iter()
            .map(|part| (*part).to_owned())
            .collect(),
            prior: PriorBlock::Present {
                entries: vec![PriorScalar {
                    key: "LaunchOptions".to_owned(),
                    value: value.map(str::to_owned),
                }],
            },
        }],
    }
}

/// Runs the step-owned undo over `undo` against `runner`.
fn undo_with(runner: &MockRunner, undo: &[Change]) -> Result<(), StepError> {
    let dir = TempDir::new().expect("temp dir");
    let facts = SystemFacts::fixture(Family::Debian);
    let cx = CoreCx::new(&facts, runner);
    let mut journal =
        Journal::open(StatePaths::new(dir.path().to_path_buf()), RunId::generate()).expect("open");
    let mut apply = ApplyCx::new(
        cx,
        ImprovementId::from_static("game.steam.launch-options"),
        runner,
        &mut journal,
    );
    restore_steam_config(undo, &mut apply)
}

#[test]
fn the_recorded_value_goes_back() {
    let runner = MockRunner::new().with_file(CONFIG, written());

    undo_with(&runner, &[change(Some("LD_PRELOAD="))]).expect("undone");

    let after = runner.file(CONFIG).expect("still there");
    assert!(after.contains("LD_PRELOAD="), "{after}");
    assert!(!after.contains("gamemoderun"), "{after}");
}

#[test]
fn a_key_the_run_added_is_removed_rather_than_emptied() {
    // An empty launch options string is a real setting Steam honours. Leaving
    // one behind would not be an undo.
    let runner = MockRunner::new().with_file(CONFIG, written());

    undo_with(&runner, &[change(None)]).expect("undone");

    let after = runner.file(CONFIG).expect("still there");
    assert!(!after.contains("LaunchOptions"), "{after}");
    assert!(after.contains("1422450"), "{after}");
}

#[test]
fn a_change_this_undo_does_not_own_is_left_alone() {
    let runner = MockRunner::new().with_file(CONFIG, written());

    undo_with(
        &runner,
        &[Change::SysctlRuntime {
            key: "vm.max_map_count".to_owned(),
            previous: "65530".to_owned(),
        }],
    )
    .expect("undone");

    assert_eq!(runner.file(CONFIG).as_deref(), Some(written().as_str()));
}

#[test]
fn a_config_that_cannot_be_read_fails_rather_than_reporting_success() {
    let runner = MockRunner::new();

    let result = undo_with(&runner, &[change(Some("LD_PRELOAD="))]);

    assert!(result.is_err(), "a missing config was reported as undone");
}
