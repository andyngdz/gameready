//! What the CLI prints, pinned against a machine that never changes.
//!
//! Every run here works against a fake root: `tests/roots/ubuntu-nvme` for the
//! system, and a Steam library built into a temporary home. Without that these
//! could only assert that some substring is present, which is how a screen
//! quietly loses a line nobody was asserting on.
//!
//! Only the output is snapshotted, never the command that produced it. The
//! paths it runs with are absolute and differ per machine, and a committed file
//! should not carry whoever's home directory took the shot.
//!
//! Reviewing a change: `cargo insta review`.

// An integration test is its own crate, so the crate-level allow in main.rs
// does not reach here. A test reports failure by panicking.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gameready_core::improvement::{ImprovementId, Privilege};
use gameready_core::journal::{digest, Change, Journal, JournalEvent, RunId, StatePaths};
use tempfile::TempDir;

/// The run the rollback snapshot undoes.
///
/// A fixed id rather than a generated one, because a `RunId` carries the time
/// it was made and the screen's first line prints it. Paired with `TZ=UTC`
/// below, this is what stops the snapshot from changing every time it runs.
const FIXED_RUN: &str = "01K2A0G8000000000000000000";

/// What the fixture run wrote, so its digest matches and the undo goes ahead
/// rather than refusing to touch a file somebody edited.
const WROTE: &str = "vm.max_map_count = 2147483642\n";

/// The fake machine every snapshot runs against.
fn fake_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/roots/ubuntu-nvme")
}

/// A home directory holding a Steam install with three games and one Proton-GE.
///
/// Built here rather than checked in because Steam's library index names its
/// own location as an absolute path, which cannot be committed: it would be
/// whichever machine wrote it.
fn fake_home() -> TempDir {
    let home = TempDir::new().expect("temp dir");
    let steam = home.path().join(".steam/steam");
    let steamapps = steam.join("steamapps");
    std::fs::create_dir_all(steamapps.join("common")).expect("steamapps");

    let games = [
        (1_091_500_u32, "Cyberpunk 2077"),
        (1_422_450, "Deadlock"),
        (2_868_840, "Slay the Spire 2"),
    ];

    let listed: Vec<String> = games
        .iter()
        .map(|(app_id, _)| format!("\t\t\t\"{app_id}\"\t\t\"1\""))
        .collect();
    std::fs::write(
        steamapps.join("libraryfolders.vdf"),
        format!(
            "\"libraryfolders\"\n{{\n\t\"0\"\n\t{{\n\t\t\"path\"\t\t\"{}\"\n\t\t\"apps\"\n\t\t{{\n{}\n\t\t}}\n\t}}\n}}\n",
            steam.display(),
            listed.join("\n"),
        ),
    )
    .expect("libraryfolders");

    for (app_id, name) in games {
        std::fs::write(
            steamapps.join(format!("appmanifest_{app_id}.acf")),
            format!(
                "\"AppState\"\n{{\n\t\"appid\"\t\t\"{app_id}\"\n\t\"name\"\t\t\"{name}\"\n\t\"installdir\"\t\t\"{name}\"\n\t\"StateFlags\"\t\t\"4\"\n}}\n"
            ),
        )
        .expect("manifest");
    }

    let tool = steam.join("compatibilitytools.d/GE-Proton11-3");
    std::fs::create_dir_all(&tool).expect("compat tool");
    std::fs::write(
        tool.join("compatibilitytool.vdf"),
        "\"compatibilitytools\"\n",
    )
    .expect("compat tool manifest");

    home
}

/// Writes a finished run into the state directory, so `rollback` has something
/// to undo.
///
/// Seeded rather than performed: every step that would leave a record here
/// needs root, and this suite runs as whoever invoked it. The one change is a
/// file in the fake home, which rollback undoes as the user.
fn seed_a_finished_run(state: &Path, home: &Path) {
    let run = RunId::parse(FIXED_RUN).expect("a fixed run id");
    let step = ImprovementId::from_static("core.sysctl.max-map-count");
    let config = home.join(".config/gameready/demo.conf");
    std::fs::create_dir_all(config.parent().expect("a parent")).expect("config dir");
    std::fs::write(&config, WROTE).expect("the file the run wrote");

    let mut journal =
        Journal::open(StatePaths::new(state.to_path_buf()), run).expect("a journal to seed");
    let events = [
        JournalEvent::RunBegin {
            argv: vec!["gameready".to_owned(), "apply".to_owned()],
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        JournalEvent::StepBegin { step: step.clone() },
        JournalEvent::Changed {
            step: step.clone(),
            change: Change::FileWritten {
                path: config,
                existed: false,
                backup: None,
                sha256_after: digest(WROTE),
                mode: 0o644,
                privilege: Privilege::User,
            },
        },
        JournalEvent::StepEnd {
            step,
            outcome: "applied".to_owned(),
        },
        JournalEvent::RunEnd {
            applied: 1,
            skipped: 0,
            failed: 0,
        },
    ];
    for event in events {
        journal.append(event).expect("appended");
    }
}

/// Which machine a snapshot run reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reads {
    /// The checked-in fake root. What almost every screen wants.
    Fixture,

    /// The machine the test is running on.
    ///
    /// Only rollback wants this. The fixture refuses every write, so a rollback
    /// against it can report nothing but its own inability to put anything
    /// back. What it undoes here is one file the seeder wrote into a temporary
    /// home, and nothing else on disk is named in the journal it reads.
    ThisMachine,
}

/// Runs gameready against the fixtures and returns what a user would see.
///
/// `seed` gets the state directory and the fake home before the run, for the
/// screens that only exist once there is something on disk to read.
///
/// The terminal settings are fixed: colour codes and a wrapped line would
/// otherwise make the output depend on the window it was taken in.
fn run_with(args: &[&str], reads: Reads, seed: impl FnOnce(&Path, &Path)) -> String {
    let state = TempDir::new().expect("temp dir");
    let home = fake_home();
    seed(state.path(), home.path());

    let mut command = Command::new(insta_cmd::get_cargo_bin("gameready"));
    if reads == Reads::Fixture {
        command.env("GAMEREADY_FAKE_ROOT", fake_root());
    }
    let output = command
        .args(args)
        .env("GAMEREADY_STATE_DIR", state.path())
        .env("GAMEREADY_GAMES_DIR", state.path().join("games"))
        .env("HOME", home.path())
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .env("COLUMNS", "100")
        // A run id carries the time it was made, and rollback prints it in the
        // reader's own zone. Without this the snapshot is whoever took it.
        .env("TZ", "UTC")
        .output()
        .expect("gameready runs");

    format!(
        "exit code: {}\n----- stdout -----\n{}\n----- stderr -----\n{}",
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}

/// Asserts the output matches, with the temporary paths in it redacted.
macro_rules! assert_output {
    ($args:expr) => {
        assert_output!($args, Reads::Fixture, |_, _| {})
    };
    ($args:expr, $reads:expr, $seed:expr) => {{
        let mut settings = insta::Settings::clone_current();
        settings.add_filter(r#"/[^\s"]*/\.tmp[A-Za-z0-9]+[^\s"]*"#, "[TMP]");
        settings.bind(|| insta::assert_snapshot!(run_with($args, $reads, $seed)));
    }};
}

#[test]
fn help_says_what_every_command_is_for() {
    assert_output!(&["--help"]);
}

#[test]
fn explain_help_says_what_it_reads_and_what_it_does_not_touch() {
    assert_output!(&["explain", "--help"]);
}

#[test]
fn doctor_reports_the_machine_and_every_step() {
    assert_output!(&["doctor"]);
}

#[test]
fn explain_lists_every_step() {
    assert_output!(&["explain"]);
}

#[test]
fn explain_a_step_that_would_run_here() {
    assert_output!(&["explain", "core.sysctl.max-map-count"]);
}

#[test]
fn explain_a_step_that_does_not_apply_to_this_machine() {
    // Disk swap, not zram, so this one has a reason rather than a plan.
    assert_output!(&["explain", "core.memory.swappiness"]);
}

#[test]
fn explain_an_unknown_step_names_the_ones_that_exist() {
    assert_output!(&["explain", "core.sysctl.max-map-conut"]);
}

#[test]
fn list_games_shows_the_shipped_profiles() {
    assert_output!(&["list-games"]);
}

#[test]
fn apply_previews_every_step_without_the_init_questions() {
    // `init` asks; `apply` is the same sweep for a terminal that cannot be
    // asked, so its dry run is the one a script reads.
    assert_output!(&["apply", "--dry-run", "--yes"]);
}

#[test]
fn rollback_puts_a_run_back_and_says_what_it_did() {
    assert_output!(&["rollback"], Reads::ThisMachine, seed_a_finished_run);
}

#[test]
fn rollback_with_nothing_to_undo_says_so_rather_than_reporting_an_empty_run() {
    assert_output!(&["rollback"]);
}

#[test]
fn selftest_applies_verifies_and_reverts_one_step() {
    // One step, and one that needs no privilege: the point here is the shape of
    // the screen, and the cycle itself is covered by the core suite.
    assert_output!(&["selftest", "--step", "core.conflicts"]);
}

#[test]
fn a_dry_run_previews_without_asking_for_a_password() {
    // The whole preview, including the Proton pin resolved against the one GE
    // build in the fake home, and no prompt: a dry run that asks for a password
    // is asking permission to change nothing.
    assert_output!(&["init", "--dry-run", "--yes"]);
}
