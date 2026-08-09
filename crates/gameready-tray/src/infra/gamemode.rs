//! Noticing when a game gameready set up starts and stops.
//!
//! gamemode is the signal, not a poll: every launch option gameready writes
//! starts with `gamemoderun`, so a configured game registering with gamemoded
//! is exactly the event the icon needs. A machine without gamemode has no
//! gameready-configured game to miss, because that is the wrapper the profiles
//! are written around.

use std::fs;
use std::path::PathBuf;

use gameready_core::games::{AppId, Catalog};
use zbus::blocking::{Connection, MessageIterator, Proxy};
use zbus::message::Type;
use zbus::zvariant::OwnedObjectPath;
use zbus::MatchRule;

use crate::infra::errors::WatchError;
use crate::tray::Activity;

/// gamemoded's well-known name, object, and interface.
const SERVICE: &str = "com.feralinteractive.GameMode";

/// The object gamemoded serves its interface on.
const OBJECT: &str = "/com/feralinteractive/GameMode";

/// Every game gamemoded currently holds, as `(pid, object path)`.
const LIST_GAMES: &str = "ListGames";

/// The variable Steam puts the appid in for the game process it launches.
const STEAM_APP_ID: &str = "SteamAppId";

/// One process gamemode is holding, as gamemoded reports it.
type HeldGame = (i32, OwnedObjectPath);

/// A live view of what gamemode is currently holding.
pub struct Watch {
    session: Connection,
    games: Proxy<'static>,
    catalog: Catalog,
}

impl Watch {
    /// Connects to gamemoded on the session bus.
    ///
    /// Fails when there is no session bus or gamemoded is not running. Both are
    /// ordinary on a machine that has not installed gamemode yet, so the caller
    /// reports this once and leaves the icon at rest.
    pub fn connect(catalog: Catalog) -> Result<Self, WatchError> {
        let session = Connection::session()?;
        let games = Proxy::new(&session, SERVICE, OBJECT, SERVICE)?;
        Ok(Self {
            session,
            games,
            catalog,
        })
    }

    /// What is playing right now, for a tray that started mid-session.
    pub fn current(&self) -> Result<Activity, WatchError> {
        let held: Vec<HeldGame> = self.games.call(LIST_GAMES, &())?;
        // The rows stay empty here: probing what gameready set for the game
        // reads Steam's config files, and this runs on the thread serving
        // D-Bus signals. The main loop fills them in.
        Ok(held
            .iter()
            .find_map(|(pid, _)| self.configured(*pid))
            .map_or(Activity::Idle, |(game, app_id)| Activity::Playing {
                game,
                app_id,
                rows: Vec::new(),
            }))
    }

    /// Blocks until gamemode's set of held games changes, and reports the new
    /// activity each time.
    ///
    /// Runs until `report` returns `false`, which is how the caller says it has
    /// gone away.
    pub fn watch(&self, mut report: impl FnMut(Activity) -> bool) -> Result<(), WatchError> {
        // One stream matching the whole interface, not one per signal name.
        // Two blocking iterators read in turn on a single thread only ever
        // advance the first: the reader sits in GameRegistered::next and never
        // reaches GameUnregistered, so the icon goes green and stays there.
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface(SERVICE)?
            .build();
        let changes = MessageIterator::for_match_rule(rule, &self.session, None)?;

        // Which signal arrived does not matter: both mean "ask gamemode again",
        // and asking is cheap next to tracking pids ourselves.
        for _ in changes {
            if !report(self.current()?) {
                return Ok(());
            }
        }
        Ok(())
    }

    /// The profile name for a held process, when gameready has one for it.
    ///
    /// A game launched under gamemode that gameready never configured is not
    /// this tray's business: a green icon claims gameready's tuning is live,
    /// and for that game it is not.
    fn configured(&self, pid: i32) -> Option<(String, AppId)> {
        let app_id = steam_app_id(pid)?;
        self.catalog
            .by_app_id(app_id)
            .map(|entry| (entry.profile.name.clone(), app_id))
    }
}

/// The Steam appid a process was launched for, read from its environment.
///
/// The environment rather than the command line: `gamemoderun` sits outside the
/// game and Proton sits inside it, so the argument vector belongs to whichever
/// wrapper happens to be the one gamemode registered. `SteamAppId` is set once
/// by Steam and inherited by every process in the tree.
fn steam_app_id(pid: i32) -> Option<AppId> {
    let environ = fs::read(PathBuf::from("/proc").join(pid.to_string()).join("environ")).ok()?;
    environ
        .split(|&byte| byte == 0)
        .filter_map(|entry| std::str::from_utf8(entry).ok())
        .find_map(|entry| entry.strip_prefix(STEAM_APP_ID)?.strip_prefix('='))
        .and_then(|value| value.trim().parse().ok())
        .map(AppId)
}

#[cfg(test)]
#[path = "gamemode_test.rs"]
mod gamemode_test;
