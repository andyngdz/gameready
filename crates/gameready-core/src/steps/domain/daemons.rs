//! Daemons that set the same things gamemode sets.

/// One daemon that competes with gamemode, and what it competes over.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompetingDaemon {
    /// The systemd unit to ask about.
    pub unit: &'static str,

    /// What it takes ownership of, in the words shown to the user.
    pub contention: &'static str,
}

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
        unit: "tuned.service",
        contention: "overrides CPU governor",
    },
    CompetingDaemon {
        unit: "power-profiles-daemon.service",
        contention: "overrides CPU governor",
    },
];
