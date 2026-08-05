//! `gameready list-games`.

use std::path::Path;

use anyhow::Result;
use gameready_core::infra::games::load_catalog;

use crate::cli::ui::GameList;

/// Lists every profile gameready can see, and which layer each came from.
pub fn run(user_games_dir: &Path) -> Result<String> {
    let (catalog, failures) = load_catalog(user_games_dir);
    Ok(GameList::new(&catalog, &failures).to_string())
}

#[cfg(test)]
#[path = "list_games_test.rs"]
mod list_games_test;
