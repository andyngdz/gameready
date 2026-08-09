//! Probing one step for a screen that changes nothing.

use crate::improvement::{CoreCx, CoreImprovement, Probe, ProbeStatus, StepError};

/// One tuning's probe result, ready to render: its short label and either what
/// probing found or why probing failed.
///
/// Lives in core rather than in a renderer because two surfaces read it now,
/// the doctor screen and the tray, and they must not disagree about what a
/// step's current state is. Each still decides its own layout: this settles
/// what was found, [`StepFinding::note`] settles the words, and nothing here
/// knows about a terminal or a panel.
#[derive(Debug)]
pub struct StepFinding {
    /// The step's terse label, for a row where the full name would crowd it.
    pub short_name: String,

    /// What probing found, or why probing itself failed.
    pub found: Result<Probe, StepError>,

    /// What the step would do, for the steps that would do something. A row
    /// reading only "would apply" tells the user this screen has an answer and
    /// then keeps it, which is the opposite of what they opened it for.
    pub would_do: Option<String>,
}

impl StepFinding {
    /// Probes one step, and asks a step that would run what it would do.
    ///
    /// The plan is worked out only for a step that would actually apply. It is
    /// the more expensive of the two calls, and for a step that is already set
    /// or ruled out it answers a question nobody asked.
    #[must_use]
    pub fn of(step: &dyn CoreImprovement, cx: &CoreCx<'_>) -> Self {
        let found = step.probe(cx);
        let would_do = matches!(found, Ok(Probe::Applicable))
            .then(|| step.plan(cx).ok().map(|plan| plan.summary))
            .flatten();
        Self {
            short_name: step.short_name().to_owned(),
            found,
            would_do,
        }
    }

    /// Which bucket this row is drawn in.
    ///
    /// A probe that could not run is [`ProbeStatus::Attention`] rather than
    /// [`ProbeStatus::Inactive`]: greying it out reads as "nothing to do here",
    /// and a step that cannot read its own state is the opposite of settled.
    #[must_use]
    pub fn status(&self) -> ProbeStatus {
        self.found
            .as_ref()
            .map_or(ProbeStatus::Attention, Probe::status)
    }

    /// What the row says after the step's name, at full length.
    #[must_use]
    pub fn note(&self) -> String {
        match &self.would_do {
            Some(plan) => format!("{}, {plan}", self.found_phrase()),
            None => self.found_phrase(),
        }
    }

    /// What probing found, or why it could not tell.
    fn found_phrase(&self) -> String {
        match &self.found {
            Ok(probe) => probe.describe(),
            Err(error) => format!("probe failed: {}", error.describe()),
        }
    }
}

#[cfg(test)]
#[path = "finding_test.rs"]
mod finding_test;
