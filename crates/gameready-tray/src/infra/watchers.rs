//! Threads that wake the main loop, and the directories they watch.
//!
//! Neither is joined: each blocks on something only the process exiting ends,
//! a D-Bus signal that may never arrive and an inotify read that waits for a
//! run nobody may start. The main loop returns moments before the process does.

use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

use gameready_core::infra::dirs;
use gameready_core::infra::games::load_catalog;

use crate::infra::journal::watch_journal;
use crate::infra::{Request, Watch};

/// Starts the gamemode watcher, if this machine has one to watch.
///
/// Not joined: the watcher blocks on a D-Bus signal that may never arrive, and
/// the only thing that ends it is the process exiting, which happens moments
/// after the main loop returns. A missing gamemode is reported once and then
/// left alone, because it is one of the tunings the tray itself reports on.
pub fn watch_for_games(requests: Sender<Request>, games: PathBuf) -> Option<JoinHandle<()>> {
    let (catalog, _failures) = load_catalog(&games);

    let watch = match Watch::connect(catalog) {
        Ok(watch) => watch,
        Err(error) => {
            tracing::info!(%error, "not watching for games");
            return None;
        }
    };

    let watching = thread::spawn(move || report_games(&watch, &requests));
    Some(watching)
}

/// Tells the main loop what gamemode is holding, now and on every change.
fn report_games(watch: &Watch, requests: &Sender<Request>) {
    if let Ok(activity) = watch.current() {
        if requests.send(Request::Playing(activity)).is_err() {
            return;
        }
    }
    if let Err(error) = watch.watch(|activity| requests.send(Request::Playing(activity)).is_ok()) {
        tracing::warn!(%error, "stopped watching for games");
    }
}

/// Watches the journal, so a run's changes reach the bar as they land.
///
/// Not joined, for the same reason as the gamemode watcher: it blocks in a
/// read that only the process exiting ends.
pub fn watch_for_changes(requests: Sender<Request>, state: PathBuf) -> Option<JoinHandle<()>> {
    Some(thread::spawn(move || {
        if let Err(error) = watch_journal(&state, || requests.send(Request::Refresh).is_ok()) {
            tracing::warn!(%error, "not watching for changes; the menu will only update on Refresh");
        }
    }))
}

/// Where the journal, backups, and logs live, matching the CLI so the tray
/// watches the file a run actually writes.
pub fn state_dir() -> PathBuf {
    dirs::state_dir().unwrap_or_default()
}

/// Where this user's own game profiles live.
///
/// An empty path when the home directory cannot be resolved: the shipped
/// profiles still load, and a game with no profile is not this tray's business
/// anyway.
pub fn user_games_dir() -> PathBuf {
    dirs::user_games_dir().unwrap_or_default()
}

#[cfg(test)]
#[path = "watchers_test.rs"]
mod watchers_test;
