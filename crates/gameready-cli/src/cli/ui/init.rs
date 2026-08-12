//! Rendering the pieces of an `init` run.

use std::fmt;

use console::style;
use gameready_core::games::AppId;
use gameready_core::steps::{CompatRank, CompatTarget};

use crate::cli::ui::layout::{Mark, Section};
use crate::cli::ui::Answers;

/// The per-game settings for the user to enter into Steam themselves.
///
/// The path taken whenever gameready is not going to close Steam, so it has to
/// carry everything the writing path would have done. A setting that only the
/// automatic path knows about is a setting the manual path silently drops.
pub struct LaunchInstructions<'a> {
    answers: &'a Answers,

    /// The Proton entries as they resolved against this machine's builds, so
    /// the user is told the build name Steam will show them rather than the
    /// profile's word for it.
    proton: &'a [CompatTarget],
}

impl<'a> LaunchInstructions<'a> {
    #[must_use]
    pub const fn new(answers: &'a Answers, proton: &'a [CompatTarget]) -> Self {
        Self { answers, proton }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.answers.targets.is_empty() && self.proton.is_empty()
    }

    /// The Proton build a game is to be pinned to, when one was worked out.
    fn proton_for(&self, app_id: AppId) -> Option<&str> {
        self.proton
            .iter()
            .find(|target| target.app_id == app_id)
            .map(|target| target.tool.as_str())
    }

    /// The build every other game is to fall back to, when one was worked out.
    fn machine_wide(&self) -> Option<&CompatTarget> {
        self.proton
            .iter()
            .find(|target| matches!(target.rank, CompatRank::MachineWide))
    }

    /// Where this one lives in Steam, which is not the per-game dialog every
    /// other line here points at.
    fn machine_wide_block<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        let Some(target) = self.machine_wide() else {
            return Ok(());
        };
        s.marked(Mark::Chosen, &target.name)?;
        s.sub("- Go to Steam > Settings > Compatibility")?;
        // Steam's own label, quoted, so it can be searched for on the screen.
        // Kept on its own line because the build name after it pushes the two
        // together past the width and wraps the name onto a line of its own.
        s.sub("- Tick \"Enable Steam Play for all other titles\"")?;
        s.sub(&format!("- Pick: {}", style(&target.tool).green()))
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
        self.machine_wide_block(&mut s)
    }
}

#[cfg(test)]
#[path = "init_test.rs"]
mod init_test;
