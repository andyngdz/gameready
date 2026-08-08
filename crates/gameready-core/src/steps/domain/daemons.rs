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

/// The daemons that own the CPU governor, so gameready's own pin would be
/// overwritten seconds later if one is live.
///
/// The CPU governor step reads this to decide whether pinning is even worth
/// offering: with one of these running, the pin loses, so the step reports the
/// conflict instead of fighting it.
pub const GOVERNOR_DAEMONS: [&str; 2] = [TUNED_UNIT, POWER_PROFILES_DAEMON_UNIT];

/// The three daemons that reliably undo what gamemode does.
///
/// All three are legitimate software a user or their distro installed on
/// purpose. Two of them are the distro's own power tooling. gameready reports
/// them and stops; disabling one would be settling a system-wide policy
/// question that is not gameready's to settle.
pub const COMPETING_DAEMONS: [CompetingDaemon; 3] = [
    CompetingDaemon {
        unit: "ananicy-cpp.service",
        contention: "overrides process priority",
    },
    CompetingDaemon {
        unit: TUNED_UNIT,
        contention: "overrides CPU governor",
    },
    CompetingDaemon {
        unit: POWER_PROFILES_DAEMON_UNIT,
        contention: "overrides CPU governor",
    },
];
