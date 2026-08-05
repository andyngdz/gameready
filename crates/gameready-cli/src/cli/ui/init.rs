//! Rendering the pieces of an `init` run.

use std::fmt;

use gameready_core::steam::GameSetup;

/// The launch options for each selected game, for the user to set themselves.
///
/// Shown when the user chose to do it by hand, and on any run where nobody is
/// there to agree to Steam being closed: a dry run, or a scripted one. gameready
/// can write these itself, but only with Steam stopped, because Steam holds its
/// config in memory and rewrites the file when it exits.
pub struct LaunchInstructions<'a> {
    selected: &'a [GameSetup],
}

impl<'a> LaunchInstructions<'a> {
    #[must_use]
    pub const fn new(selected: &'a [GameSetup]) -> Self {
        Self { selected }
    }

    /// Whether there is anything for the user to do.
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

        writeln!(f, "\nPer-game settings")?;
        writeln!(
            f,
            "  Paste each line into the game's launch options. Steam has to be closed"
        )?;
        writeln!(
            f,
            "  for gameready to set them for you, so it did not: run `gameready init`"
        )?;
        writeln!(f, "  and pick the first option if you would rather it did.")?;

        for setup in self.selected {
            let options = setup.launch_options();
            if options.is_empty() {
                continue;
            }
            writeln!(f)?;
            writeln!(f, "  {}", setup.game.name)?;
            writeln!(
                f,
                "    Steam > right click the game > Properties > Launch Options"
            )?;
            writeln!(f, "    {options}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "init_test.rs"]
mod init_test;
