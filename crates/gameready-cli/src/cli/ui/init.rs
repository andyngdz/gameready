//! Rendering the pieces of an `init` run.

use std::fmt;

use console::style;
use gameready_core::games::AppId;

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::questions::Answers;

/// The per-game settings for the user to enter into Steam themselves.
///
/// The path taken whenever gameready is not going to close Steam, so it has to
/// carry everything the writing path would have done. A setting that only the
/// automatic path knows about is a setting the manual path silently drops.
pub struct LaunchInstructions<'a> {
    answers: &'a Answers,
}

impl<'a> LaunchInstructions<'a> {
    #[must_use]
    pub const fn new(answers: &'a Answers) -> Self {
        Self { answers }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.answers.targets.is_empty() && self.answers.proton.is_empty()
    }

    /// The Proton build a game is to be pinned to, when one was worked out.
    fn proton_for(&self, app_id: AppId) -> Option<&str> {
        self.answers
            .proton
            .iter()
            .find(|target| target.app_id == app_id)
            .map(|target| target.tool.as_str())
    }
}

impl fmt::Display for LaunchInstructions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return Ok(());
        }

        let mut s = Section::new(f);
        s.blank()?;
        s.title("Per-game settings")?;
        for setup in &self.answers.selected {
            let options = setup.launch_options();
            let proton = self.proton_for(setup.game.app_id);
            if options.is_empty() && proton.is_none() {
                continue;
            }

            s.marked(Mark::Chosen, &setup.game.name)?;
            s.sub("- Go to Steam > right click the game > Properties")?;
            if !options.is_empty() {
                s.sub(&format!(
                    "- Under General, put this into Launch Options: {}",
                    style(&options).green()
                ))?;
            }
            if let Some(tool) = proton {
                s.sub(&format!(
                    "- Under Compatibility, tick the box and pick: {}",
                    style(tool).green()
                ))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "init_test.rs"]
mod init_test;
