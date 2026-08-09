//! Keeping exactly one tray on the bar.
//!
//! A D-Bus name rather than a lock file: the bus releases it the moment the
//! owner dies, however it died, so there is no stale lock to reason about and
//! no cleanup path to get wrong. It is also the same bus the tray already
//! needs, so a machine where this cannot work has no tray to duplicate.

use zbus::blocking::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

use crate::infra::errors::SingleError;

/// The well-known name one running tray owns.
const NAME: &str = "io.github.andyngdz.gameready.Tray";

/// Ownership of the tray's well-known name.
///
/// Held for as long as the tray runs. Dropping it closes the connection, which
/// is what hands the name back.
pub struct Held {
    _session: Connection,
}

/// Whether this process may put a tray on the bar.
pub enum Claim {
    /// It may: nothing else held the name, and this process holds it now.
    Ours(Held),

    /// It may not: another tray is already running.
    Taken,
}

/// Claims the right to be the tray, without waiting for a turn.
///
/// Never queues. A second launch, from the app grid while autostart's copy is
/// still alive, should say so and leave rather than sit invisibly waiting for
/// the first one to quit.
pub fn claim() -> Result<Claim, SingleError> {
    let session = Connection::session()?;
    // DoNotQueue on a live connection, not Builder::name: the builder queues
    // for a taken name, so a second launch would sit invisibly waiting for the
    // first one to quit instead of saying it is already running.
    //
    // zbus reports a name someone else holds as an error rather than as a
    // reply, so the ordinary "another tray is running" case arrives down the
    // Err arm and is not a failure at all.
    match session.request_name_with_flags(NAME, RequestNameFlags::DoNotQueue.into()) {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => {
            Ok(Claim::Ours(Held { _session: session }))
        }
        Ok(RequestNameReply::Exists | RequestNameReply::InQueue) => Ok(Claim::Taken),
        Err(zbus::Error::NameTaken) => Ok(Claim::Taken),
        Err(error) => Err(SingleError::Bus(error)),
    }
}

#[cfg(test)]
#[path = "single_test.rs"]
mod single_test;
