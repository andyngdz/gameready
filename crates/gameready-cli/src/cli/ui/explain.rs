//! What one step is, and what it would do to this machine.

use std::fmt;

use console::style;
use gameready_core::improvement::{CoreCx, CoreImprovement, Privilege, Probe, StepPlan};
use gameready_core::steps::core_steps;

use crate::cli::ui::UNDO;
use crate::cli::ui::colors::Section;

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
        let applicable = matches!(probe, Ok(Probe::Applicable));
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
            found,
            plan,
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
        s.title(&format!("{}  {}", style(&self.id).bold(), self.name))?;
        s.labelled("Why", &self.rationale)?;
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
        s.labelled(UNDO, "gameready rollback")?;
        s.end()
    }
}

/// Every step gameready knows, as one line each.
pub struct StepIndex {
    steps: Vec<(String, String)>,
}

impl StepIndex {
    /// The whole catalog, in the order a run would work through it.
    #[must_use]
    pub fn all() -> Self {
        Self {
            steps: core_steps()
                .iter()
                .map(|step| (step.id().to_string(), step.name().to_owned()))
                .collect(),
        }
    }
}

impl fmt::Display for StepIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = Section::new(f);
        s.title("Steps")?;
        for (id, name) in &self.steps {
            s.indented(&format!("{:<28}{name}", style(id).bold().to_string()))?;
        }
        s.blank()?;
        s.indented("Run `gameready explain <id>` for what one of them would do here.")?;
        s.end()
    }
}

#[cfg(test)]
#[path = "explain_test.rs"]
mod explain_test;
