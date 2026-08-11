//! What the tray's adapters can fail with.

/// What went wrong turning the artwork into pixels.
///
/// Every variant is a bug in the shipped asset or a machine out of memory, not
/// something a user can act on, so the tray reports it once and carries on with
/// a themed icon name instead of refusing to start.
#[derive(Debug, thiserror::Error)]
pub enum IconError {
    /// The shipped SVG did not parse.
    #[error("the shipped controller artwork could not be parsed")]
    Artwork(#[from] resvg::usvg::Error),

    /// A pixmap of the requested size could not be allocated.
    #[error("could not allocate a {size}x{size} pixmap")]
    Allocate {
        /// The edge length that failed.
        size: u32,
    },

    /// A drawn pixmap could not be encoded as the PNG dbusmenu wants.
    #[error("could not encode a {size}x{size} icon as PNG")]
    Encode {
        /// The edge length that failed.
        size: u32,
    },
}

/// Why the tray could not find out whether another one is already running.
///
/// Only a session bus that refused. Without one there is no tray at all, so the
/// caller reports this and stops rather than risking a second icon.
#[derive(Debug, thiserror::Error)]
pub enum SingleError {
    /// The session bus refused the connection or the name request.
    #[error(transparent)]
    Bus(#[from] zbus::Error),
}

/// Why the tray cannot see what gamemode is holding.
///
/// Almost always "gamemode is not installed", which is one of the tunings the
/// tray itself reports on. The icon stays at rest and everything else works.
#[derive(Debug, thiserror::Error)]
pub enum WatchError {
    /// The session bus refused, or gamemoded is not on it.
    #[error(transparent)]
    Bus(#[from] zbus::Error),

    /// The journal could not be watched, so a run's changes would go unnoticed
    /// until the next time the menu is opened.
    #[error("could not watch the journal for changes")]
    Watch(#[from] rustix::io::Errno),
}

/// Why clicking a row could not open the terminal.
///
/// None of these is fatal: the row was asked to update Proton-GE and the tray
/// could not even start the terminal that would, so the main loop logs it and
/// carries on serving the menu.
#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    /// No terminal could be found, so there is nothing to run the command in.
    #[error("no terminal could be found to run the update in")]
    NoTerminal,

    /// The `gameready` binary is not on the PATH, so the terminal would have
    /// nothing to run.
    #[error("the gameready command is not on the PATH")]
    GamereadyNotFound,

    /// A terminal was found but refused to start.
    #[error("the terminal {program} could not be started")]
    Spawn {
        /// Which terminal was found but would not run.
        program: String,

        /// What the operating system said.
        #[source]
        source: std::io::Error,
    },
}
