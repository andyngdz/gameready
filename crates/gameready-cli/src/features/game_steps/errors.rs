//! Errors raised while constructing per-game steps.

use gameready_core::improvement::ImprovementId;

/// Why a per-game step could not be built for this machine.
#[derive(Debug, thiserror::Error)]
pub(crate) enum GameStepBuildError {
    /// No Steam installation or account configuration was available.
    #[error("could not find a Steam installation to test against")]
    SteamUnavailable,

    /// Steam has no installed game for this step to modify.
    #[error("no installed games were found, so there is nothing for `{step}` to write")]
    NoGames { step: ImprovementId },
}
