//! Adapters: the artwork renderer and the StatusNotifierItem.

mod errors;
mod gamemode;
mod icon;
mod ink;
mod items;
pub(crate) mod journal;
mod single;
pub(crate) mod sni;
mod watchers;

pub use gamemode::Watch;
pub use ink::Ink;
pub use single::{claim, Claim};
pub use sni::Indicator;
pub use watchers::{state_dir, user_games_dir, watch_for_changes, watch_for_games};

use crate::tray::Activity;

/// What a watcher or a clicked menu item asks the main loop to do.
///
/// A menu handler runs on the thread serving D-Bus and the gamemode watcher
/// runs on its own, so both post one of these and return rather than sweeping
/// the machine from a thread that has something else to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Read the machine again now.
    Refresh,

    /// A configured game started or stopped.
    Playing(Activity),

    /// Stop the tray.
    Quit,
}
