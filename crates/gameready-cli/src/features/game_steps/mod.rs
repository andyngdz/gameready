//! Per-game step construction.

mod errors;
mod service;

pub(crate) use errors::GameStepBuildError;
pub(crate) use service::{build_game_step, is_game_step};
