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

/// Everything `core.pkg.tools` installs.
///
/// gamemode is what actually tunes anything. mangohud is here so the user can
/// answer "did that help", which is the only honest way to justify the rest of
/// what gameready does; whether it appears in a launch option is a separate
/// question the run asks.
///
/// gamescope is deliberately absent. It solves a real class of windowing
/// problems, but no shipped profile invokes it, and installing 9MB that nothing
/// runs is a surprise rather than a service.
///
/// Both names are the same on pacman, apt, and dnf. They still go through
/// [`PackageSpec`] rather than a bare string, because a package that does not
/// exist on a family has to be reportable and that machinery has to exist.
pub const GAMING_TOOLS: [GamingTool; 2] = [
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
];
