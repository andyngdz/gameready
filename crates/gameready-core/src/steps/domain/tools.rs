//! The three tools a Linux gaming setup is expected to have.

use crate::improvement::PackageSpec;

/// One tool, the executable it puts on `PATH`, and what it is called on each
/// distro family.
///
/// The binary name and the package name are separate fields because they differ
/// often enough to matter: the `gamemode` package installs `gamemoded`, and a
/// probe that looked for `gamemode` on `PATH` would reinstall it forever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamingTool {
    /// The executable the package puts on `PATH`, and the probe.
    pub binary: &'static str,

    /// What to install, per distro family.
    pub spec: PackageSpec,

    /// What the tool does, for the plan and summary screens. One sentence, for
    /// someone who has not heard of it.
    pub what: &'static str,
}

/// Rough installed sizes, so the plan screen can total them honestly.
///
/// Measured from the Ubuntu 26.04 archive on 2026-08-05. They are estimates
/// used for a size line the user reads before agreeing, not a contract, and
/// they differ per distro by a few megabytes.
const GAMEMODE_BYTES: u64 = 1_100_000;
const MANGOHUD_BYTES: u64 = 5_400_000;
const GAMESCOPE_BYTES: u64 = 8_900_000;

/// Everything `core.pkg.tools` installs.
///
/// All three carry the same name on pacman, apt, and dnf. That is not a
/// guarantee for the future, which is why they still go through
/// [`PackageSpec`] rather than a bare string: `gamescope` is absent from Debian
/// 12 entirely, and the machinery that reports that has to exist anyway.
pub const GAMING_TOOLS: [GamingTool; 3] = [
    GamingTool {
        // The package is `gamemode`; the daemon it installs is `gamemoded`.
        binary: "gamemoded",
        spec: PackageSpec::uniform("gamemode", GAMEMODE_BYTES),
        what: "raises the CPU governor and process priority while a game runs, \
               and puts both back when it exits",
    },
    GamingTool {
        binary: "mangohud",
        spec: PackageSpec::uniform("mangohud", MANGOHUD_BYTES),
        what: "draws frame rate, frame times, and temperatures over the game, \
               so a change can be measured rather than guessed at",
    },
    GamingTool {
        binary: "gamescope",
        spec: PackageSpec::uniform("gamescope", GAMESCOPE_BYTES),
        what: "runs a game in its own nested compositor, which fixes alt-tab \
               and resolution handling that the desktop compositor gets wrong",
    },
];
