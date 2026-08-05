//! Telling whether Steam is running, and asking it to stop.

use std::time::{Duration, Instant};

use crate::exec::{Cmd, CommandRunner};
use crate::steam::SteamError;

/// The client's process name, which is what `pgrep -x` matches against.
const STEAM_PROCESS: &str = "steam";

/// How long to wait for Steam to finish shutting down before giving up.
///
/// Steam flushes its config, cloud state, and shader cache on the way out, and
/// on a slow disk with a large library that is not instant. Twenty seconds is
/// long enough for that and short enough that a Steam which is never going to
/// exit does not hang the run.
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(20);

/// How often to re-check while waiting.
const POLL: Duration = Duration::from_millis(500);

/// Whether the Steam client is running right now.
///
/// This is the load-bearing check for every config write: Steam holds its
/// config in memory and rewrites the file when it exits, so a write made while
/// it runs is discarded without a word.
pub fn is_running(runner: &dyn CommandRunner) -> bool {
    let probe = Cmd::user("pgrep").arg("-x").arg(STEAM_PROCESS);
    runner
        .run_allowing_failure(&probe)
        .is_ok_and(|output| output.code == 0)
}

/// Asks Steam to quit, and waits until it has.
///
/// `steam -shutdown` is the client's own graceful exit, the same one the menu
/// uses, so Steam writes its config out properly instead of losing whatever it
/// held in memory. Killing the process would take that write away and lose
/// settings the user made this session.
pub fn shutdown(runner: &dyn CommandRunner) -> Result<(), SteamError> {
    if !is_running(runner) {
        return Ok(());
    }

    let quit = Cmd::user(STEAM_PROCESS).arg("-shutdown");
    runner
        .run_allowing_failure(&quit)
        .map_err(|source| SteamError::Shutdown {
            detail: source.to_string(),
        })?;

    let deadline = Instant::now() + SHUTDOWN_TIMEOUT;
    while Instant::now() < deadline {
        if !is_running(runner) {
            return Ok(());
        }
        std::thread::sleep(POLL);
    }

    Err(SteamError::StillRunning {
        waited: SHUTDOWN_TIMEOUT,
    })
}

#[cfg(test)]
#[path = "process_test.rs"]
mod process_test;
