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

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

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

/// Runs gameready against the fixtures and returns what a user would see.
///
/// The terminal settings are fixed: colour codes and a wrapped line would
/// otherwise make the output depend on the window it was taken in.
fn gameready(args: &[&str]) -> String {
    let state = TempDir::new().expect("temp dir");
    let home = fake_home();

    let output = Command::new(insta_cmd::get_cargo_bin("gameready"))
        .args(args)
        .env("GAMEREADY_FAKE_ROOT", fake_root())
        .env("GAMEREADY_STATE_DIR", state.path())
        .env("GAMEREADY_GAMES_DIR", state.path().join("games"))
        .env("HOME", home.path())
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .env("COLUMNS", "100")
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
    ($args:expr) => {{
        let mut settings = insta::Settings::clone_current();
        settings.add_filter(r#"/[^\s"]*/\.tmp[A-Za-z0-9]+[^\s"]*"#, "[TMP]");
        settings.bind(|| insta::assert_snapshot!(gameready($args)));
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
fn a_dry_run_previews_without_asking_for_a_password() {
    // The whole preview, including the Proton pin resolved against the one GE
    // build in the fake home, and no prompt: a dry run that asks for a password
    // is asking permission to change nothing.
    assert_output!(&["init", "--dry-run", "--yes"]);
}
