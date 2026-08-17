//! Feature services used by the CLI interface.

mod game_steps;
mod rollback;
mod step_selection;

pub(crate) use game_steps::{build_game_step, is_game_step, GameStepBuildError};
pub(crate) use rollback::{preview_rows, rollback_plan, PreviewRow};
pub(crate) use step_selection::{find_step, select_steps, select_steps_including_games};
