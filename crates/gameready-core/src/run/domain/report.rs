//! What a run produced.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::improvement::{Dependency, ImprovementId, Outcome, OutcomeKind};
use crate::journal::RunId;

use super::preflight::PreflightReport;

/// Whether a run may change anything.
///
/// An enum rather than a `dry_run: bool` so the apply path has to name which
/// mode it is in, and adding a third mode later is a compile error at every
/// site that decides rather than a silently wrong branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Probe, plan, and stop. Nothing is written.
    DryRun,
    /// Probe, plan, install prerequisites, apply, verify.
    Apply,
}

impl Mode {
    /// Whether this mode is allowed to change the system.
    #[must_use]
    pub const fn mutates(self) -> bool {
        matches!(self, Self::Apply)
    }
}

/// How one step ended, with enough context to render it.
#[derive(Debug, Serialize, Deserialize)]
pub struct StepReport {
    /// Which step.
    pub step: ImprovementId,

    /// Its human title, copied so the report renders without the step itself.
    pub name: String,

    /// How it ended.
    pub outcome: Outcome,
}

impl StepReport {
    /// Records how one step ended, copying the title off the step itself.
    #[must_use]
    pub fn for_step(step: &dyn crate::improvement::CoreImprovement, outcome: Outcome) -> Self {
        Self {
            step: step.id(),
            name: step.name().to_owned(),
            outcome,
        }
    }
}

/// Everything one invocation did.
///
/// The single value `--json` serialises and the summary screen renders, so the
/// two can never disagree about what happened.
#[derive(Debug, Serialize, Deserialize)]
pub struct RunReport {
    /// The run this describes.
    pub run: RunId,

    /// Which mode it ran in.
    pub mode: Mode,

    /// One entry per selected step, in the order they were attempted.
    pub steps: Vec<StepReport>,

    /// Prerequisites that were missing and had to be installed first.
    pub installed_dependencies: Vec<String>,

    /// Wall clock time for the whole run.
    pub took: Duration,
}

impl RunReport {
    /// How many steps applied successfully.
    #[must_use]
    pub fn applied(&self) -> usize {
        self.steps
            .iter()
            .filter(|report| matches!(report.outcome, Outcome::Applied { .. }))
            .count()
    }

    /// How many steps failed.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.steps
            .iter()
            .filter(|report| report.outcome.is_failure())
            .count()
    }

    /// How this run should end the process.
    #[must_use]
    pub fn status(&self) -> RunStatus {
        if self.failed() > 0 {
            return RunStatus::StepFailed;
        }
        if self.steps.is_empty() {
            return RunStatus::NothingApplicable;
        }
        RunStatus::Clean
    }
}

/// How a run ended, at the process boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// Everything that ran succeeded.
    Clean,
    /// At least one step failed.
    StepFailed,
    /// Nothing on this system was applicable, so nothing ran.
    NothingApplicable,
    /// The user interrupted the run.
    Interrupted,
}

impl RunStatus {
    /// The process exit code.
    ///
    /// `2` is skipped: it belongs to argument and config errors, which the CLI
    /// reports before a run ever starts. `130` is the shell convention for
    /// SIGINT.
    #[must_use]
    pub const fn code(self) -> i32 {
        match self {
            Self::Clean => 0,
            Self::StepFailed => 1,
            Self::NothingApplicable => 3,
            Self::Interrupted => 130,
        }
    }
}

/// Progress from a run in flight.
///
/// The executor emits these; the CLI renders them. Passing an event stream
/// rather than letting the executor print is what keeps this crate free of any
/// terminal dependency, so a step cannot print or prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEvent {
    /// Probing has started for one step.
    ///
    /// Carries its place in the sweep, because probing is the one phase with
    /// nothing to show for itself while it runs: no step has an outcome yet,
    /// so the count is all the progress there is to report.
    Probing {
        step: ImprovementId,
        done: usize,
        total: usize,
    },

    /// A step the probe ruled out is being held open, because a step it names
    /// in `requires()` is going to run and may change the answer.
    Deferred {
        step: ImprovementId,
        name: String,
        reason: String,
        waiting_on: Vec<ImprovementId>,
    },

    /// Every step has been probed and the plan is settled.
    Planned { applicable: usize, skipped: usize },

    /// A held-open step is being probed a second time, now that the step named
    /// here has finished.
    Reprobing {
        step: ImprovementId,
        after: ImprovementId,
    },

    /// Dependencies have been resolved. The CLI renders the preflight screen
    /// from this.
    DependenciesResolved { report: PreflightReport },

    /// Installing prerequisites.
    InstallingDependencies { count: usize },

    /// Prerequisites installed.
    DependenciesInstalled { newly_installed: Vec<String> },

    /// A step is about to change something.
    Applying { step: ImprovementId, name: String },

    /// A sub-phase within a step changed status. Steps that involve multiple
    /// visible operations (download, verify, extract) emit these so the CLI
    /// can render a per-phase checklist instead of a single spinner.
    StepProgress {
        step: ImprovementId,
        message: String,
    },

    /// A step finished.
    Finished {
        step: ImprovementId,
        name: String,
        kind: OutcomeKind,
        detail: Option<String>,
    },
}

/// A prerequisite that is missing and can be installed.
///
/// Carries the step that wants it so the pre-flight screen can say which
/// improvement each install is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingDependency {
    /// The step that declared it.
    pub wanted_by: ImprovementId,

    /// The prerequisite itself, including the text shown to the user.
    pub dependency: Dependency,
}
