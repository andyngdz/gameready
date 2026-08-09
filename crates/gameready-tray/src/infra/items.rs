//! Turning one row into one menu item.
//!
//! Split from the `Tray` impl so that file stays about what the panel asks for
//! and this one about what each answer looks like.

use ksni::menu::{MenuItem, StandardItem, SubMenu};

use crate::infra::errors::IconError;
use crate::infra::icon;
use crate::infra::ink::Ink;
use crate::infra::sni::Indicator;
use crate::tray::Row;

/// A submenu holding a group of rows, titled with how many of them hold.
pub(super) fn folder(title: &str, rows: &[Row]) -> MenuItem<Indicator> {
    SubMenu {
        label: format!("{title} ({} of {})", held(rows), rows.len()),
        submenu: rows.iter().map(tuning).collect(),
        ..SubMenu::default()
    }
    .into()
}

/// One tuning as a menu row: a coloured dot, the name, and what it is set to.
pub(super) fn tuning(row: &Row) -> MenuItem<Indicator> {
    let ink = Ink::for_status(row.status);
    let icon_data = icon::dot(ink).unwrap_or_else(|error| {
        report(&error);
        Vec::new()
    });
    StandardItem {
        label: format!("{row}"),
        icon_data,
        // Enabled even though clicking does nothing. A disabled dbusmenu item
        // is drawn greyed and never highlights on hover, so a menu of them
        // reads as broken next to every other tray on the bar. Read-only is
        // kept by the empty handler, not by refusing to light up.
        ..StandardItem::default()
    }
    .into()
}

/// A line the user reads and cannot click.
pub(super) fn note(text: &str) -> MenuItem<Indicator> {
    StandardItem {
        label: text.to_owned(),
        enabled: false,
        ..StandardItem::default()
    }
    .into()
}

/// How many of these tunings are in place right now.
pub(super) fn held(rows: &[Row]) -> usize {
    rows.iter().filter(|row| row.is_set()).count()
}

/// Says once that the artwork could not be drawn.
///
/// Diagnostics rather than user-facing text: there is nothing the user can do
/// about a broken shipped asset, and the tray keeps working without it.
pub(super) fn report(error: &IconError) {
    tracing::warn!(%error, "drawing the tray artwork failed");
}

#[cfg(test)]
#[path = "items_test.rs"]
mod items_test;
