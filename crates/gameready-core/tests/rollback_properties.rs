//! The property every rollback has to hold, checked against generated runs.
//!
//! The example-based tests each pin one sequence somebody thought of. This one
//! generates sequences nobody thought of and asserts the same thing about all
//! of them: whatever a run recorded, undoing it puts the machine back.

// An integration test is its own crate, so the crate-level allow in lib.rs does
// not reach here. A test reports failure by panicking either way.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use gameready_core::exec::CommandRunner;
use gameready_core::improvement::Privilege;
use gameready_core::infra::exec::MockRunner;
use gameready_core::journal::{digest, load, Change, Journal, JournalEvent, RunId, StatePaths};
use gameready_core::rollback::{execute, plan};
use gameready_core::steam::{restore_sections, PriorBlock, PriorScalar, PriorSection};
use proptest::prelude::*;
use tempfile::TempDir;

const CONFIG: &str = "/steam/config/config.vdf";
const SECTION: [&str; 6] = [
    "InstallConfigStore",
    "Software",
    "Valve",
    "Steam",
    "CompatToolMapping",
    "1422450",
];

/// A file gameready creates, named so two generated changes never collide.
fn dropin(index: usize) -> String {
    format!("/etc/sysctl.d/9{index}-gameready.conf")
}

/// A kernel knob, and where it reads back from.
fn knob(index: usize) -> (String, String) {
    let key = format!("vm.generated_{index}");
    let path = format!("/proc/sys/{}", key.replace('.', "/"));
    (key, path)
}

/// One mutation a generated run can make.
#[derive(Debug, Clone)]
enum Step {
    /// Writes a file that was not there, undone by deleting it.
    WriteFile { index: usize, contents: String },
    /// Moves a kernel knob, undone by putting the old value back.
    SetSysctl { index: usize, to: String },
    /// Sets a key in a config Steam owns, undone by putting the key back.
    SetSteamKey { to: String },
}

fn steps() -> impl Strategy<Value = Vec<Step>> {
    let step = prop_oneof![
        (0usize..4, "[a-z]{1,8}").prop_map(|(index, contents)| Step::WriteFile {
            index,
            contents: format!("{contents}\n"),
        }),
        (0usize..4, 0u32..999).prop_map(|(index, to)| Step::SetSysctl {
            index,
            to: to.to_string(),
        }),
        "[a-z]{1,8}".prop_map(|to| Step::SetSteamKey { to }),
    ];
    prop::collection::vec(step, 1..8)
}

/// The Steam config as the machine starts, before any generated run.
fn steam_config(tool: &str) -> String {
    format!(
        "\"InstallConfigStore\"\n{{\n\t\"Software\"\n\t{{\n\t\t\"Valve\"\n\t\t{{\n\t\t\t\"Steam\"\
         \n\t\t\t{{\n\t\t\t\t\"CompatToolMapping\"\n\t\t\t\t{{\n\t\t\t\t\t\"1422450\"\
         \n\t\t\t\t\t{{\n\t\t\t\t\t\t\"name\"\t\t\"{tool}\"\n\t\t\t\t\t}}\n\t\t\t\t}}\
         \n\t\t\t}}\n\t\t}}\n\t}}\n}}\n"
    )
}

/// The Steam config as the vdf renderer writes it.
///
/// Seeded in canonical form on purpose. The renderer sorts keys and normalises
/// indentation, so a hand-written fixture changes shape the first time anything
/// rewrites it, and a before-and-after comparison would fail on formatting
/// rather than on anything rollback did.
fn canonical_steam_config(tool: &str) -> String {
    let text = steam_config(tool);
    let unchanged = PriorSection {
        section: SECTION.iter().map(|part| (*part).to_owned()).collect(),
        prior: PriorBlock::Present {
            entries: vec![PriorScalar {
                key: "name".to_owned(),
                value: Some(tool.to_owned()),
            }],
        },
    };
    restore_sections(&text, &[unchanged]).expect("the fixture parses")
}

/// A machine with every knob at a known value and Steam pinned to a known tool.
fn machine() -> MockRunner {
    let mut runner = MockRunner::new().with_file(CONFIG, canonical_steam_config("stock_tool"));
    for index in 0..4 {
        let (key, path) = knob(index);
        runner = runner.with_file(&path, "100\n");
        for value in 0..1000u32 {
            runner = runner.where_command_writes(
                format!("sudo sysctl -w {key}={value}"),
                &path,
                format!("{value}\n"),
            );
        }
    }
    runner
}

/// Every file on the machine, for comparing before against after.
fn snapshot(runner: &MockRunner) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = runner
        .paths()
        .into_iter()
        .filter_map(|path| Some((path.display().to_string(), runner.file(&path)?)))
        .collect();
    files.sort();
    files
}

/// Reads what a Steam key holds right now, for recording it before a write.
fn steam_prior(runner: &MockRunner) -> PriorSection {
    let text = runner.file(CONFIG).expect("the config is seeded");
    let name = text
        .lines()
        .find(|line| line.contains("\"name\""))
        .and_then(|line| line.rsplit('"').nth(1).map(str::to_owned));
    PriorSection {
        section: SECTION.iter().map(|part| (*part).to_owned()).collect(),
        prior: PriorBlock::Present {
            entries: vec![PriorScalar {
                key: "name".to_owned(),
                value: name,
            }],
        },
    }
}

/// Performs one generated step, journalling its undo record first.
///
/// Records before mutating, the same order the engine enforces, so a generated
/// run is undoable at every point the same way a real one is.
fn perform(runner: &MockRunner, journal: &mut Journal, step: &Step) {
    let recorded = match step {
        Step::WriteFile { index, contents } => {
            let path = dropin(*index);
            if runner.file(&path).is_some() {
                return;
            }
            Change::FileWritten {
                path: path.clone().into(),
                sha256_after: digest(contents),
                mode: 0o644,
                privilege: Privilege::Root,
            }
        }
        Step::SetSysctl { index, to } => {
            let (key, path) = knob(*index);
            let previous = runner.file(&path).expect("seeded").trim().to_owned();
            if previous == *to {
                return;
            }
            Change::SysctlRuntime { key, previous }
        }
        Step::SetSteamKey { to } => {
            let text = runner.file(CONFIG).expect("seeded");
            if text.contains(&format!("\"{to}\"")) {
                return;
            }
            Change::SteamConfigWritten {
                path: CONFIG.into(),
                sections: vec![steam_prior(runner)],
            }
        }
    };

    journal
        .append(JournalEvent::Changed {
            step: gameready_core::improvement::ImprovementId::from_static("test.generated"),
            change: recorded,
        })
        .expect("appends");

    match step {
        Step::WriteFile { index, contents } => {
            runner
                .write_file(
                    std::path::Path::new(&dropin(*index)),
                    contents,
                    Privilege::Root,
                )
                .expect("writes");
        }
        Step::SetSysctl { index, to } => {
            let (key, _) = knob(*index);
            let cmd = gameready_core::exec::Cmd::root("sysctl")
                .arg("-w")
                .arg(format!("{key}={to}"));
            runner.run(&cmd).expect("runs");
        }
        Step::SetSteamKey { to } => {
            let text = runner.file(CONFIG).expect("seeded");
            let rewritten = text.replace("stock_tool", to);
            let rewritten = if rewritten == text {
                canonical_steam_config(to)
            } else {
                rewritten
            };
            runner
                .write_file(std::path::Path::new(CONFIG), &rewritten, Privilege::User)
                .expect("writes");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Whatever a run did, undoing it puts every file back where it was.
    #[test]
    fn a_generated_run_rolls_back_to_where_it_started(steps in steps()) {
        let dir = TempDir::new().expect("temp dir");
        let paths = StatePaths::new(dir.path().to_path_buf());
        let runner = machine();
        let before = snapshot(&runner);

        let run = RunId::generate();
        let mut journal = Journal::open(paths.clone(), run).expect("journal opens");
        for step in &steps {
            perform(&runner, &mut journal, step);
        }
        drop(journal);

        let records = load(&paths.journal()).expect("reads");
        let undo_plan = plan(&records, run).expect("plans");
        let mut rollback_journal =
            Journal::open(paths.clone(), RunId::generate()).expect("journal opens");
        let report =
            execute(&undo_plan, &runner, &mut rollback_journal).expect("rollback runs");

        prop_assert_eq!(report.failed(), 0, "rollback failed: {:?}", report.undos);
        prop_assert_eq!(snapshot(&runner), before, "steps were {:?}", steps);
    }
}
