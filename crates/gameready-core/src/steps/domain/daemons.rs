//! Daemons that set the same things gamemode sets.

/// One daemon that competes with gamemode, and what it competes over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetingDaemon {
    /// The systemd unit to ask about.
    pub unit: &'static str,

    /// What it takes ownership of, in the words shown to the user.
    pub contention: &'static str,
}

/// tuned's unit. Names the CPU governor among the things it manages.
pub const TUNED_UNIT: &str = "tuned.service";

/// power-profiles-daemon's unit, the GNOME and KDE default power tool.
pub const POWER_PROFILES_DAEMON_UNIT: &str = "power-profiles-daemon.service";

/// A daemon that drives the CPU governor on its own schedule, and whether
/// gamemode coordinates with it rather than being overridden by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernorDaemon {
    /// The systemd unit to ask about.
    pub unit: &'static str,

    /// Whether gamemode holds this daemon's performance profile over D-Bus for
    /// the length of a game. When it does, the daemon is no rival to gamemode:
    /// gamemode is raising the governor and this daemon is how it does it. Only
    /// power-profiles-daemon has that integration; tuned has none.
    pub cooperates_with_gamemode: bool,
}

/// The daemons that own the CPU governor, so gameready's own static pin would be
/// overwritten seconds later if one is live.
///
/// The CPU governor step reads this to decide whether pinning is even worth
/// offering: with one of these running, the pin loses, so the step reports the
/// conflict instead of fighting it. With gamemode present, a daemon it
/// cooperates with is not a conflict, because gamemode is already driving it.
pub const GOVERNOR_DAEMONS: [GovernorDaemon; 2] = [
    GovernorDaemon {
        unit: TUNED_UNIT,
        cooperates_with_gamemode: false,
    },
    GovernorDaemon {
        unit: POWER_PROFILES_DAEMON_UNIT,
        cooperates_with_gamemode: true,
    },
];

/// The daemons that reliably undo what gamemode does while a game runs.
///
/// Both are software a user or their distro installed on purpose, and both set
/// things gamemode also sets without coordinating with it: ananicy-cpp reorders
/// process priorities, tuned drives the CPU governor on its own schedule.
/// gameready reports them and stops; disabling one would be settling a
/// system-wide policy question that is not gameready's to settle.
///
/// power-profiles-daemon is deliberately absent. Modern gamemode holds the
/// performance profile through its D-Bus API for the length of a game and
/// releases it after, so the two cooperate rather than fight, and telling a
/// user to disable it would lose that coordinated switch and their laptop's
/// power management with it. It stays in `GOVERNOR_DAEMONS`, where the contest
/// is against gameready's own static pin rather than against gamemode.
pub const COMPETING_DAEMONS: [CompetingDaemon; 2] = [
    CompetingDaemon {
        unit: "ananicy-cpp.service",
        contention: "overrides process priority",
    },
    CompetingDaemon {
        unit: TUNED_UNIT,
        contention: "overrides CPU governor",
    },
];
