//! What the run is about to do, shown before it does any of it.

use std::fmt;

use console::style;
use gameready_core::facts::PackageManagerKind;
use gameready_core::run::{InstallConsent, Mode, RunPlan};
use gameready_core::steam::{GameSetup, Overlay};
use itertools::Itertools as _;

use crate::cli::ui::install::approx_size;
use crate::cli::ui::layout::Section;
use crate::cli::ui::{tunings, Answers, PER_GAME, SYSTEM};

/// The command that puts the whole run back.
const ROLLBACK: &str = "gameready rollback";

/// Why the password is being asked for, and that it is the last thing asked.
const ONE_PASSWORD: &str = "I need your password once, for the changes outside your home folder. \
                            After that I won't ask you anything else.";

/// What a dry run is about to do instead, which is nothing.
const DRY_RUN: &str = "Dry run: nothing below actually happens.";

/// The agreed plan, printed before the first change.
///
/// One line per kind of change, so the last thing a user reads before the
/// password prompt is a list they can check against what they answered.
pub struct InitPlan<'a> {
    found: &'a [GameSetup],
    answers: &'a Answers,
    plan: &'a RunPlan,
    packages: PackageManagerKind,
    mode: Mode,
}

impl<'a> InitPlan<'a> {
    #[must_use]
    pub const fn new(
        found: &'a [GameSetup],
        answers: &'a Answers,
        plan: &'a RunPlan,
        packages: PackageManagerKind,
        mode: Mode,
    ) -> Self {
        Self {
            found,
            answers,
            plan,
            packages,
            mode,
        }
    }

    /// The games this run touches, or why it touches none.
    fn games(&self) -> String {
        if self.found.is_empty() {
            return style("no games found").dim().to_string();
        }
        if self.answers.selected.is_empty() {
            return style("none picked").dim().to_string();
        }
        self.answers
            .selected
            .iter()
            .map(|setup| style(&setup.game.name).bold().to_string())
            .join(", ")
    }

    /// What will be installed, and how much of it, or `None` when the run
    /// agreed to install nothing.
    fn install(&self) -> Option<String> {
        if !matches!(self.answers.consent, InstallConsent::Granted) {
            return None;
        }
        let installs = self.plan.installs(self.packages);
        if installs.is_empty() {
            return None;
        }
        let bytes: u64 = installs.iter().map(|install| install.approx_bytes).sum();
        let listed = installs
            .iter()
            .map(|install| install.package.as_str())
            .join(", ");
        if bytes == 0 {
            return Some(listed);
        }
        Some(format!(
            "{listed} {}",
            style(format!("· {}", approx_size(bytes))).dim()
        ))
    }

    /// How many system tunings will run, and which parts of the machine they
    /// are about.
    fn system(&self) -> Option<String> {
        let count = self.plan.pending.len();
        if count == 0 {
            return None;
        }
        let noun = tunings(count);
        let mut areas: Vec<&'static str> = Vec::new();
        for tag in self.plan.pending.iter().flat_map(|step| step.tags()) {
            let label = tag.label();
            if !areas.contains(&label) {
                areas.push(label);
            }
        }
        if areas.is_empty() {
            return Some(format!("{count} {noun}"));
        }
        Some(format!(
            "{count} {noun} {}",
            style(format!("· {}", areas.join(", "))).dim()
        ))
    }

    /// What the run writes into Steam, counted per kind.
    fn per_game(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.answers.targets.is_empty() {
            parts.push(format!("launch options ×{}", self.answers.targets.len()));
        }
        if !self.answers.proton.is_empty() {
            parts.push(format!("Proton pin ×{}", self.answers.proton.len()));
        }
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// Whether anything in this run reaches outside the user's own files.
    ///
    /// The same question the escalation asks a moment later, answered from the
    /// same place: a run of nothing but Steam config never prompts, and
    /// promising a password prompt that never comes is its own kind of wrong.
    fn needs_password(&self) -> bool {
        self.mode.mutates() && self.plan.needs_root()
    }

    /// Every row, in the order the run will carry them out.
    fn rows<W: fmt::Write>(&self, s: &mut Section<'_, W>) -> fmt::Result {
        s.labelled("Games", &self.games())?;
        if let Some(install) = self.install() {
            s.labelled("Install", &install)?;
        }
        if let Some(system) = self.system() {
            s.labelled(SYSTEM, &system)?;
        }
        if let Some(per_game) = self.per_game() {
            s.labelled(PER_GAME, &per_game)?;
        }
        s.labelled("Overlay", Self::overlay(self.answers.overlay))?;
        s.labelled(
            "Undo",
            &format!(
                "{} {}",
                style(ROLLBACK).bold(),
                style("· any time, no reboot").dim()
            ),
        )
    }

    /// Which way the overlay answer went.
    const fn overlay(overlay: Overlay) -> &'static str {
        match overlay {
            Overlay::Show => "on",
            Overlay::Hide => "off",
        }
    }
}

impl fmt::Display for InitPlan<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.blank()?;
        s.title("Here's what I'll do")?;
        self.rows(&mut s)?;
        s.blank()?;

        if self.mode.mutates() {
            if self.needs_password() {
                s.indented(&style(ONE_PASSWORD).dim().to_string())?;
            }
        } else {
            s.indented(&style(DRY_RUN).dim().to_string())?;
        }
        s.end()
    }
}

#[cfg(test)]
#[path = "plan_test.rs"]
mod plan_test;
