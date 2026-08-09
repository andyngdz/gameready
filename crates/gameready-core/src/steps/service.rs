//! The catalog of built-in improvements.

use std::path::PathBuf;

use crate::improvement::{CoreImprovement, ImprovementId};
use crate::steps::use_cases::{
    Conflicts, CpuGovernor, GamemodeConfig, GamingTools, IoScheduler, MaxMapCount, ProtonGe,
    ScxLavd, ScxPpa, ShaderCache, SplitLock, SteamLaunchOptions, SteamProton, Swappiness,
    VmLatency,
};

/// Every system-wide improvement gameready ships, in the order they apply.
///
/// Order matters where one step's effect changes what another probes, so this
/// is a list rather than a set. Steps that genuinely depend on each other say
/// so through `requires()` as well; this ordering is the tie-breaker for the
/// ones that merely read better in a particular sequence.
#[must_use]
pub fn core_steps() -> Vec<Box<dyn CoreImprovement>> {
    vec![
        // Conflicts first: what it finds explains why gamemode may look like it
        // is doing nothing, and the user should read that before the steps that
        // install gamemode and defer to it.
        Box::new(Conflicts),
        Box::new(MaxMapCount),
        Box::new(SplitLock),
        Box::new(VmLatency),
        Box::new(IoScheduler),
        Box::new(Swappiness),
        Box::new(GamingTools),
        // After the tools, so a machine that just got gamemode is re-probed
        // through `requires()` rather than told to run gameready twice.
        Box::new(GamemodeConfig::detect()),
        Box::new(ShaderCache::detect()),
        Box::new(ProtonGe::detect()),
        // The repository before the step that installs from it, so a single
        // run can go from an Ubuntu box with no scx to a loaded scheduler.
        Box::new(ScxPpa),
        // After the tools, because it may install 180MB and the user reads the
        // one install screen for the whole run before any of it is fetched.
        Box::new(ScxLavd),
        Box::new(CpuGovernor),
    ]
}

/// The per-game steps, for listing in `explain`.
///
/// Built with an empty config and no targets: only their identity is read here.
/// `init` constructs the real ones from the games the user picked, so these are
/// never applied.
#[must_use]
pub fn game_steps() -> Vec<Box<dyn CoreImprovement>> {
    vec![
        Box::new(SteamLaunchOptions::new(PathBuf::new(), Vec::new())),
        Box::new(SteamProton::new(PathBuf::new(), Vec::new())),
    ]
}

/// Finds one step by id, for `apply --step` and `explain`.
#[must_use]
pub fn find_core_step(id: &ImprovementId) -> Option<Box<dyn CoreImprovement>> {
    core_steps().into_iter().find(|step| &step.id() == id)
}

#[cfg(test)]
#[path = "service_test.rs"]
mod service_test;
