//! Names the systemd queries are built from.

/// The one binary this feature drives.
pub const SYSTEMCTL: &str = "systemctl";

/// Turns a unit on at boot and starts it in the same call.
pub const ENABLE: &str = "enable";

/// Turns a unit off at boot and stops it in the same call.
pub const DISABLE: &str = "disable";

/// Makes `enable` and `disable` act on the running system too, not only on the
/// next boot.
pub const NOW: &str = "--now";

/// Stops a running unit, leaving its enablement alone.
pub const STOP: &str = "stop";

/// Starts a unit without touching its enablement.
pub const RESTART: &str = "restart";

/// Asks whether a unit starts at boot. Also the existence check: a name with no
/// unit file behind it fails here with an empty answer.
pub const IS_ENABLED: &str = "is-enabled";

/// Asks whether a unit is running right now.
pub const IS_ACTIVE: &str = "is-active";

/// `is-active` prints exactly this when the unit is running.
pub const ACTIVE: &str = "active";

/// The `is-enabled` answers that mean the unit will come up on its own.
///
/// `static` and `indirect` units have no enablement switch of their own but are
/// pulled in by something that does, so for "will this run" they count the same
/// as `enabled`.
pub const ENABLED_ANSWERS: [&str; 5] = [
    "enabled",
    "enabled-runtime",
    "static",
    "indirect",
    "generated",
];
