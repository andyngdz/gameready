//! The StatusNotifierItem the panel talks to.
//!
//! The only file that knows about ksni. Everything it draws comes in as a
//! [`Snapshot`], so the menu can be built and asserted on without a session
//! bus anywhere in the test.

use std::sync::mpsc::Sender;

use ksni::menu::{MenuItem, StandardItem};
use ksni::{Icon, OfflineReason, ToolTip, Tray};

use crate::infra::ink::Ink;
use crate::infra::items::{folder, held, note, report};
use crate::infra::{icon, Request};
use crate::tray::{Activity, Snapshot};

/// The D-Bus name this item registers under.
const ID: &str = "gameready";

/// What the panel shows on hover, and what a menu with no rows yet says.
const TITLE: &str = "gameready";

/// The submenu holding every machine-wide tuning.
const SYSTEM: &str = "System tunings";

/// The item the user clicks to read the machine again now.
const REFRESH: &str = "Refresh now";

/// The item the user clicks to stop the tray.
const QUIT: &str = "Quit";

/// What the menu says before the first sweep lands.
///
/// A state of its own rather than an empty menu: an empty menu on a machine
/// where every tuning is already set looks identical, and one of those means
/// "you are fine" while the other means "I have not looked yet".
const CHECKING: &str = "Checking this machine...";

/// What to say when there is nowhere on the bar to draw an icon.
pub const NO_HOST: &str = "no system tray on the session bus yet. On GNOME this \
needs the AppIndicator extension; on other desktops, a panel with tray support.";

/// The panel indicator, holding whatever the last sweep found.
pub struct Indicator {
    /// The rows to draw, or nothing yet.
    snapshot: Option<Snapshot>,

    /// Whether a configured game is running, which is the icon's colour.
    activity: Activity,

    /// The colour the controller rests in, from what the user chose.
    resting: Ink,

    /// Where a clicked menu item sends its request. Clicks must not block the
    /// menu, so the handler does nothing but post one of these.
    requests: Sender<Request>,
}

impl Indicator {
    /// A tray that has not read the machine yet.
    #[must_use]
    pub const fn new(resting: Ink, requests: Sender<Request>) -> Self {
        Self {
            snapshot: None,
            activity: Activity::Idle,
            resting,
            requests,
        }
    }

    /// Replaces what the menu shows with what the latest sweep found.
    pub fn show(&mut self, snapshot: Snapshot) {
        self.snapshot = Some(snapshot);
    }

    /// Records whether a configured game is running, repainting the icon.
    pub fn playing(&mut self, activity: Activity) {
        self.activity = activity;
    }

    /// The colour the controller is drawn in right now.
    const fn ink(&self) -> Ink {
        match self.activity {
            Activity::Idle => self.resting,
            Activity::Playing { .. } => Ink::Live,
        }
    }

    /// The controller, or an empty list if the artwork could not be rendered.
    ///
    /// A tray with no pixmap falls back to the themed icon name, which is
    /// worse than the artwork and much better than no tray at all.
    fn artwork(&self) -> Vec<Icon> {
        match icon::controller(self.ink()) {
            Ok(icons) => icons,
            Err(error) => {
                report(&error);
                Vec::new()
            }
        }
    }

    /// The top level: one line per subject, with the detail one hover away.
    ///
    /// Thirteen tunings at the top level is a menu nobody reads. The label
    /// carries the count so the common question is answered without opening
    /// anything.
    fn rows(&self) -> Vec<MenuItem<Self>> {
        match &self.snapshot {
            None => vec![note(CHECKING)],
            Some(Snapshot::Unreadable { reason }) => vec![note(reason)],
            Some(Snapshot::Ready { rows }) => {
                let mut items = vec![folder(SYSTEM, rows)];
                // The game's own two tunings, under its name. Only while it is
                // running, and only when there is something to say: an empty
                // submenu is a promise the row cannot keep.
                if let Activity::Playing { game, rows, .. } = &self.activity {
                    if !rows.is_empty() {
                        items.push(folder(game, rows));
                    }
                }
                items
            }
        }
    }

    /// Posts a request, ignoring a closed channel.
    ///
    /// A closed channel means the main loop has already left, so the tray is
    /// seconds from being torn down and there is nothing to report to.
    fn ask(&self, request: Request) {
        let _ = self.requests.send(request);
    }
}

impl Tray for Indicator {
    fn id(&self) -> String {
        ID.to_owned()
    }

    /// Keeps waiting when the bar has nowhere to put an icon.
    ///
    /// Autostart runs at login, often before the desktop's tray host is up, so
    /// giving up here would mean the indicator never appears on most logins.
    /// Waiting is also right for a shell restart, which takes the watcher down
    /// and brings it back seconds later.
    fn watcher_offline(&self, reason: OfflineReason) -> bool {
        tracing::warn!(?reason, "{NO_HOST}");
        true
    }

    fn watcher_online(&self) {
        tracing::info!("a system tray appeared, showing the indicator");
    }

    fn title(&self) -> String {
        TITLE.to_owned()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        self.artwork()
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: TITLE.to_owned(),
            description: summary(self.snapshot.as_ref()),
            ..ToolTip::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items = self.rows();
        if let Activity::Playing { game, rows, .. } = &self.activity {
            // Named here only when its own submenu is not drawn, so a running
            // game is never invisible and never announced twice.
            if rows.is_empty() {
                items.push(note(&format!("{game} is running")));
            }
        }
        items.push(MenuItem::Separator);
        items.push(
            StandardItem {
                label: REFRESH.to_owned(),
                activate: Box::new(|this: &mut Self| this.ask(Request::Refresh)),
                ..StandardItem::default()
            }
            .into(),
        );
        items.push(
            StandardItem {
                label: QUIT.to_owned(),
                activate: Box::new(|this: &mut Self| this.ask(Request::Quit)),
                ..StandardItem::default()
            }
            .into(),
        );
        items
    }
}

/// The one line the panel shows on hover.
fn summary(snapshot: Option<&Snapshot>) -> String {
    match snapshot {
        None => CHECKING.to_owned(),
        Some(Snapshot::Unreadable { reason }) => reason.clone(),
        Some(Snapshot::Ready { rows }) => {
            format!("{} of {} tunings in place", held(rows), rows.len())
        }
    }
}

#[cfg(test)]
#[path = "sni_test.rs"]
mod sni_test;
