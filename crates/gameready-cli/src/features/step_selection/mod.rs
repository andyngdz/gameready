//! Select core and per-game steps for a command.

mod service;

pub(crate) use service::{find_step, select_steps, select_steps_including_games};
