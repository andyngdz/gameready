//! Wiring: build a read-only runner, put an indicator on the bar, and keep it
//! fed with what the machine currently says.

// A test reports failure by panicking, so expect, unwrap, and panic are its
// assertion mechanism. The deny in Cargo.toml targets the paths that run on a
// user's machine.
#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used, clippy::panic))]

mod infra;
mod tray;

use std::env;
use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc::{self, Sender};

use ksni::blocking::{Handle, TrayMethods as _};
use tracing_subscriber::EnvFilter;

use gameready_core::infra::exec::RealRunner;

use crate::infra::{
    claim, state_dir, user_games_dir, watch_for_changes, watch_for_games, Claim, Indicator, Ink,
    Request,
};
use crate::tray::{sweep, sweep_game, Activity};

/// What the tray reports when the user has not asked for more.
///
/// A background process that says nothing when it cannot draw its icon or find
/// gamemode is a process the user cannot debug, so warnings are on by default
/// and `RUST_LOG` turns the rest up.
/// zbus is quieter than `warn` here on purpose. Claiming the single-instance
/// name on a live connection makes it warn that method calls arriving before
/// the object server exists may be lost, and that connection serves nothing
/// and never will. `RUST_LOG` still turns it back up.
const DEFAULT_LOG: &str = "warn,zbus=error";

/// What a second launch says before leaving the first one alone.
const ALREADY_RUNNING: &str = "already running; its icon is on the bar.";

fn main() -> ExitCode {
    start_logging();

    // Before anything else: a second icon for the same machine is worse than
    // no second tray, and the app grid entry sits right next to the autostart
    // copy that is probably already running.
    let held = match claim() {
        Ok(Claim::Ours(held)) => held,
        Ok(Claim::Taken) => {
            eprintln!("gameready-tray: {ALREADY_RUNNING}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("gameready-tray: {error}");
            return ExitCode::from(2);
        }
    };

    let runner = match RealRunner::detect() {
        Ok(runner) => runner,
        Err(error) => {
            eprintln!("gameready-tray: {error}");
            return ExitCode::from(2);
        }
    };

    let (requests, incoming) = mpsc::channel();
    let handle = match show(requests.clone()) {
        Ok(handle) => handle,
        Err(error) => {
            eprintln!("gameready-tray: {error}");
            return ExitCode::from(3);
        }
    };

    // Held rather than joined: see watch_for_games.
    let games = user_games_dir();
    let watcher = watch_for_games(requests.clone(), games.clone());
    let journal = watch_for_changes(requests, state_dir());
    serve(&runner, &handle, &incoming, &games);

    handle.shutdown().wait();
    drop(watcher);
    drop(journal);
    drop(held);
    ExitCode::SUCCESS
}

/// Sends diagnostics to stderr. Without this every tracing call in the crate
/// goes nowhere, including the ones that say the tray could not draw its icon.
fn start_logging() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_LOG.into()))
        .with_writer(std::io::stderr)
        .init();
}

/// Puts an indicator on the bar and hands back the handle that drives it.
///
/// Assumes the bar will have somewhere to put an icon even if it does not yet.
/// Autostart fires at login, ahead of the desktop's tray host, and failing here
/// would mean the indicator never appears on most logins.
/// [`Indicator::watcher_offline`] says so and keeps waiting.
fn show(requests: Sender<Request>) -> Result<Handle<Indicator>, ksni::Error> {
    let resting = Ink::resting(env::var(Ink::VARIABLE).ok().as_deref());
    Indicator::new(resting, requests)
        .assume_sni_available(true)
        .spawn()
}

/// Keeps the indicator fed until it is asked to stop.
///
/// The sweep runs here rather than on the thread serving D-Bus: it shells out a
/// dozen times and would freeze an open menu. Returns when the tray is asked to
/// quit or the indicator goes away.
fn serve(
    runner: &RealRunner,
    handle: &Handle<Indicator>,
    incoming: &mpsc::Receiver<Request>,
    games: &Path,
) {
    let mut sweeping = true;
    loop {
        if sweeping {
            let snapshot = sweep(runner);
            // The one line that says the tray noticed. A daemon that re-probes
            // silently gives nobody a way to tell "it never woke" from "it woke
            // and nothing had changed".
            tracing::debug!("read the machine");
            if handle.update(|tray| tray.show(snapshot)).is_none() {
                return;
            }
        }
        match next(incoming) {
            Some(Request::Refresh) => sweeping = true,
            Some(Request::Playing(activity)) => {
                // A game starting is not a reason to re-probe the machine:
                // nothing a system tuning reads changes because gamemode
                // picked up a process. Its own two tunings are read here,
                // because that means reading Steam's config files.
                sweeping = false;
                let activity = with_game_rows(runner, activity, games);
                if handle.update(|tray| tray.playing(activity)).is_none() {
                    return;
                }
            }
            Some(Request::Quit) | None => return,
        }
    }
}

/// Blocks until there is a reason to do something.
///
/// No timeout: a tuning does not change unless something changes it, and every
/// change lands in the journal, which `watch_for_changes` is watching. Waking
/// up on a timer to re-probe a machine nobody touched spends a dozen
/// subprocesses to learn nothing. `None` means the tray should stop.
fn next(incoming: &mpsc::Receiver<Request>) -> Option<Request> {
    incoming.recv().ok()
}

/// Fills in what gameready set for the game that just started.
///
/// The watcher reports which game without reading anything: it runs on the
/// thread serving D-Bus signals, and this reads Steam's config files.
fn with_game_rows(runner: &RealRunner, activity: Activity, games: &Path) -> Activity {
    match activity {
        Activity::Idle => Activity::Idle,
        Activity::Playing { game, app_id, .. } => Activity::Playing {
            rows: sweep_game(runner, app_id, games),
            game,
            app_id,
        },
    }
}
