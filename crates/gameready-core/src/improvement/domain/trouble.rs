//! What a step that went wrong has to say for itself.

use crate::improvement::domain::outcome::{Outcome, RollbackStatus, SkipReason};

/// How a trouble that left nothing behind says so.
const NOTHING_LEFT: &str = "It broke before anything was written, so nothing was left behind.";

/// How a trouble that changed nothing says so.
const NOTHING_CHANGED: &str = "Nothing on this machine was changed.";

/// The three things a step that went wrong always states.
///
/// The shape comes from the only question a user really has when something
/// fails: is my machine broken now? An error message answers that for whoever
/// wrote the step and for nobody else. So every trouble says what broke, what
/// state the machine is in because of it, and the one command that fixes it
/// when a command does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trouble {
    /// What broke, in the words of whatever broke it.
    pub broke: String,

    /// What state the machine is in now, said plainly rather than left to be
    /// worked out from the error.
    pub now: String,

    /// The one command that fixes it. `None` when nothing needs fixing, which
    /// is the good news and worth saying by leaving the line out.
    pub fix: Option<Remedy>,
}

/// The one command a trouble ends on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Remedy {
    /// This run's own undo. The words are here and the command is not: only
    /// the CLI holds the run id to spell it with.
    Rollback { lead: String },

    /// Something only the user can decide to run, because it turns off
    /// something they turned on.
    Yours { lead: String, command: String },
}

impl Outcome {
    /// What went wrong, for the outcomes where something did.
    ///
    /// `None` for every ending a user has no work to do about, declining a step
    /// and a dry run included. Those are choices, not troubles, and putting
    /// three lines under them would bury the two that matter.
    #[must_use]
    pub fn trouble(&self) -> Option<Trouble> {
        match self {
            Self::Failed { error, rolled_back } => Some(failure(error, rolled_back)),
            Self::Skipped { reason } => skip(reason),
            Self::Applied { .. } | Self::AlreadyApplied { .. } | Self::NotApplicable { .. } => None,
        }
    }
}

/// A step that broke, and what its undo did about it.
fn failure(error: &str, rolled_back: &RollbackStatus) -> Trouble {
    match rolled_back {
        RollbackStatus::NotAttempted => Trouble {
            broke: error.to_owned(),
            now: NOTHING_LEFT.to_owned(),
            fix: None,
        },
        RollbackStatus::Succeeded => Trouble {
            broke: error.to_owned(),
            now: "I undid the partial change, so this is exactly as it was before the run."
                .to_owned(),
            fix: None,
        },
        RollbackStatus::Failed { detail } => Trouble {
            broke: error.to_owned(),
            now: format!("Undoing it failed too ({detail}), so the change may still be in place."),
            fix: Some(Remedy::Rollback {
                lead: "This retries the undo:".to_owned(),
            }),
        },
    }
}

/// A step that stood down, for the reasons a user can act on.
fn skip(reason: &SkipReason) -> Option<Trouble> {
    match reason {
        // `with` is not repeated here: the detail above has just named it, and
        // saying it twice in three lines reads as a template rather than as a
        // sentence somebody wrote.
        SkipReason::Conflict { detail, yours, .. } => Some(Trouble {
            broke: detail.clone(),
            now: "I left it alone rather than fight it for the setting.".to_owned(),
            fix: yours.clone().map(|command| Remedy::Yours {
                lead: "Your call. I will not turn it off behind your back:".to_owned(),
                command,
            }),
        }),
        SkipReason::CouldNotTell { detail } => Some(Trouble {
            broke: format!("I could not check: {detail}"),
            now: format!("I skip rather than guess. {NOTHING_CHANGED}"),
            fix: None,
        }),
        SkipReason::MissingDependency { name, detail } => Some(Trouble {
            broke: format!("I could not get {name}: {detail}"),
            now: format!("The step needs it. {NOTHING_CHANGED}"),
            fix: None,
        }),
        SkipReason::DependencyFailed { on } => Some(Trouble {
            broke: format!("{on} failed earlier in this run."),
            now: "This one builds on it, so running it would build on a state that is not there."
                .to_owned(),
            fix: None,
        }),
        SkipReason::UserDeclined | SkipReason::DryRun => None,
    }
}

#[cfg(test)]
#[path = "trouble_test.rs"]
mod trouble_test;
