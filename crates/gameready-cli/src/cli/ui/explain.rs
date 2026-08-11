//! What one step is, and what it would do to this machine.

use std::fmt;

use console::style;
use gameready_core::improvement::{CoreCx, CoreImprovement, Privilege, Probe, StepPlan};
use gameready_core::steps::{core_steps, game_steps};

use crate::cli::ui::layout::Section;
use crate::cli::ui::{PER_GAME, SYSTEM, UNDO};

/// The command that puts a run back, shown at the foot of every explanation.
const ROLLBACK_COMMAND: &str = "gameready rollback";

/// Everything `explain` found out about one step, ready to print.
///
/// The plan is separate from what probing found, because a step that does not
/// apply here has no plan to show. Printing an empty "would do" under it reads
/// as gameready being about to do nothing rather than as the step not being for
/// this machine.
pub struct StepExplanation {
    pub id: String,
    pub name: String,
    pub rationale: String,
    pub privilege: Privilege,

    /// The payoff, as opposed to the mechanism in `rationale`. `None` when the
    /// step has no benefit separable from how it works.
    pub gains: Option<String>,

    /// A word of reassurance after the rollback command, or `None`.
    pub undo_note: Option<String>,

    /// What probing found on this machine, in the step's own words.
    pub found: String,

    /// The plan, present only when the step would actually run here.
    pub plan: Option<StepPlan>,
}

impl StepExplanation {
    /// Everything one step has to say about itself and about this machine.
    #[must_use]
    pub fn of(step: &dyn CoreImprovement, cx: &CoreCx<'_>) -> Self {
        let probe = step.probe(cx);
        // An outdated install is still work a run would do, so it is applicable.
        let applicable = matches!(probe, Ok(Probe::Applicable | Probe::UpdateAvailable { .. }));
        let mut found = probe.map_or_else(
            |error| format!("could not tell: {error}"),
            |probe| probe.describe(),
        );

        // A step that would run here but cannot work out how is a real answer,
        // so it goes in the line the user reads rather than into a missing
        // section they have to notice the absence of.
        let plan = match (applicable, step.plan(cx)) {
            (true, Ok(plan)) => Some(plan),
            (true, Err(error)) => {
                found = format!("{found}, but the plan could not be worked out: {error}");
                None
            }
            (false, _) => None,
        };

        Self {
            id: step.id().to_string(),
            name: step.name().to_owned(),
            rationale: step.rationale().to_owned(),
            privilege: step.privilege(),
            gains: step.gains().map(str::to_owned),
            undo_note: step.undo_note().map(str::to_owned),
            found,
            plan,
        }
    }

    /// The rollback command, with the step's reassurance in parentheses when it
    /// has one.
    fn undo(&self) -> String {
        match &self.undo_note {
            None => ROLLBACK_COMMAND.to_owned(),
            Some(note) => format!("{ROLLBACK_COMMAND} ({note})"),
        }
    }

    /// What the privilege means to someone deciding whether to run this.
    const fn needs(&self) -> &'static str {
        match self.privilege {
            Privilege::Root => "your password, because it changes something outside your home",
            Privilege::User => "nothing extra, it only touches your own files",
        }
    }
}

impl fmt::Display for StepExplanation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.title(&format!(
            "{}  {}",
            style(&self.name).bold(),
            style(&self.id).dim()
        ))?;
        s.labelled("Why", &self.rationale)?;
        if let Some(gains) = &self.gains {
            s.labelled("Gets", gains)?;
        }
        s.blank()?;
        s.labelled("Needs", self.needs())?;
        s.labelled("Here", &self.found)?;

        if let Some(plan) = &self.plan {
            s.blank()?;
            s.labelled("Would do", &plan.summary)?;
            for action in &plan.actions {
                s.labelled("", &action.describe())?;
            }
        }

        s.blank()?;
        s.labelled(UNDO, &self.undo())?;
        s.end()
    }
}

/// Every step gameready knows, grouped as the explain index shows them.
pub struct StepIndex {
    system: Vec<(String, String)>,
    per_game: Vec<(String, String)>,
}

impl StepIndex {
    /// The whole catalog, in the order a run would work through it.
    #[must_use]
    pub fn all() -> Self {
        let cards = |steps: Vec<Box<dyn CoreImprovement>>| {
            steps
                .iter()
                .map(|step| (step.id().to_string(), step.blurb().to_owned()))
                .collect()
        };
        Self {
            system: cards(core_steps()),
            per_game: cards(game_steps()),
        }
    }

    /// The id-column width shared by both groups, so their blurbs line up.
    fn column(&self) -> usize {
        self.system
            .iter()
            .chain(&self.per_game)
            .map(|(id, _)| console::measure_text_width(id))
            .max()
            .unwrap_or(0)
    }

    /// One named group: a heading, then a row per step.
    fn group<W: fmt::Write>(
        &self,
        s: &mut Section<'_, W>,
        heading: &str,
        rows: &[(String, String)],
        column: usize,
    ) -> fmt::Result {
        s.heading(heading)?;
        for (id, blurb) in rows {
            s.entry(id, blurb, column)?;
        }
        Ok(())
    }
}

impl fmt::Display for StepIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.title(&format!(
            "{} tunings, in the order a run works through them",
            self.system.len()
        ))?;
        let column = self.column();
        self.group(&mut s, SYSTEM, &self.system, column)?;
        s.blank()?;
        self.group(&mut s, PER_GAME, &self.per_game, column)?;
        s.blank()?;
        s.indented("Ask about any one: gameready explain core.io.scheduler")?;
        s.end()
    }
}

#[cfg(test)]
#[path = "explain_test.rs"]
mod explain_test;
