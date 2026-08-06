//! Rendering the pieces of an `init` run.

use std::fmt;

use console::style;
use gameready_core::steam::GameSetup;

use crate::cli::ui::colors::Section;

/// Launch options for the user to copy into Steam.
pub struct LaunchInstructions<'a> {
    selected: &'a [GameSetup],
}

impl<'a> LaunchInstructions<'a> {
    #[must_use]
    pub const fn new(selected: &'a [GameSetup]) -> Self {
        Self { selected }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.selected
            .iter()
            .all(|setup| setup.launch_options().is_empty())
    }
}

impl fmt::Display for LaunchInstructions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        let mut s = Section::new(f);
        s.title("Per-game settings")?;
        for setup in self.selected {
            let options = setup.launch_options();
            if options.is_empty() {
                continue;
            }
            s.marked("-", &setup.game.name)?;
            s.sub("- Go to Steam > right click the game > Properties > Launch Options")?;
            s.sub(&format!(
                "- Put this into Launch Options: {}",
                style(&options).green()
            ))?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "init_test.rs"]
mod init_test;
