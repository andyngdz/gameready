//! What a step is handed to do its work.

use crate::exec::CommandRunner;
use crate::facts::SystemFacts;
use crate::improvement::errors::StepError;
use crate::journal::{Change, Journal, JournalEvent, RunId};
use crate::pkg::PackageManager;

use super::identity::ImprovementId;

/// Read-only context for a system-wide step.
///
/// Carries no way to mutate anything. `probe`, `plan`, and `verify` take this,
/// which is what makes "probing must not change the system" a property the type
/// system holds rather than a rule reviewers have to remember.
#[derive(Clone, Copy)]
pub struct CoreCx<'a> {
    /// What was probed about this machine before the run started.
    pub facts: &'a SystemFacts,

    /// The only route to the system. Reads are free; mutations belong in
    /// `apply` and must go through [`ApplyCx::mutate`].
    pub runner: &'a dyn CommandRunner,

    /// The distro's package tooling, for steps that install something of their
    /// own rather than declaring it through `dependencies()`.
    ///
    /// Optional because a step that touches no packages must still be testable
    /// without standing one up, and because the doctor and rollback paths build
    /// a context without ever installing anything. A step that needs one and
    /// finds `None` reports that it cannot tell rather than assuming.
    pub packages: Option<&'a dyn PackageManager>,
}

impl<'a> CoreCx<'a> {
    /// Builds a context from probed facts and a runner, with no package
    /// tooling. The common case: most steps write files and set kernel
    /// parameters.
    #[must_use]
    pub const fn new(facts: &'a SystemFacts, runner: &'a dyn CommandRunner) -> Self {
        Self {
            facts,
            runner,
            packages: None,
        }
    }

    /// Adds the package tooling a step needs to install something itself.
    #[must_use]
    pub const fn with_packages(mut self, packages: &'a dyn PackageManager) -> Self {
        self.packages = Some(packages);
        self
    }
}

/// Mutating context, handed only to `apply` and `rollback`.
///
/// The single way to change the system is [`ApplyCx::mutate`], which makes the
/// undo record durable before the change happens. A step that reaches for the
/// runner to mutate has bypassed that and is a bug; the clippy
/// `disallowed_methods` entry in `clippy.toml` catches the common shape.
///
/// Generic over the read-only context so the same apply machinery serves both
/// system-wide steps and per-game steps, which need different read-only state.
pub struct ApplyCx<'a, C> {
    /// The read-only context, still available for probing mid-apply.
    pub cx: C,
    step: ImprovementId,
    runner: &'a dyn CommandRunner,
    journal: &'a mut Journal,
    recorded: Vec<Change>,
}

impl<'a, C> ApplyCx<'a, C> {
    /// Wraps a read-only context with the runner and journal for one step.
    pub fn new(
        cx: C,
        step: ImprovementId,
        runner: &'a dyn CommandRunner,
        journal: &'a mut Journal,
    ) -> Self {
        Self {
            cx,
            step,
            runner,
            journal,
            recorded: Vec::new(),
        }
    }

    /// Which step is applying.
    #[must_use]
    pub const fn step(&self) -> &ImprovementId {
        &self.step
    }

    /// Which run this belongs to.
    ///
    /// Steps stamp it into every file they create, so `doctor` can map a
    /// leftover file back to the run that made it even when the journal is
    /// gone.
    #[must_use]
    pub const fn run(&self) -> RunId {
        self.journal.run()
    }

    /// Reads through the runner without recording anything.
    ///
    /// For a step that needs to look at the system mid-apply, such as reading a
    /// value back before overwriting it. Mutating through this bypasses the
    /// journal and is what `disallowed_methods` is configured to catch.
    #[must_use]
    pub const fn reader(&self) -> &'a dyn CommandRunner {
        self.runner
    }

    /// Performs one mutation, with its undo record on disk first.
    ///
    /// The ordering is the safety property of the whole design: `change` is
    /// appended to the journal and fsync'd, and only then does `mutate_fn` run.
    /// An interrupt at any point therefore leaves the system in a state that is
    /// a prefix of a fully undoable sequence, never ahead of one.
    ///
    /// If journalling fails, `mutate_fn` never runs and nothing is changed.
    pub fn mutate<T, F>(&mut self, change: Change, mutate_fn: F) -> Result<T, StepError>
    where
        F: FnOnce(&dyn CommandRunner) -> Result<T, StepError>,
    {
        self.journal.append(JournalEvent::Changed {
            step: self.step.clone(),
            change: change.clone(),
        })?;

        // Recorded before the mutation runs, not after. A command that fails
        // partway may still have changed something, and the undo record is the
        // only way to find out and put it back.
        self.recorded.push(change);

        mutate_fn(self.runner)
    }

    /// Every change recorded during this step, in the order performed.
    ///
    /// The executor hands these to `rollback` when verification fails, so a
    /// step undoes exactly what it did rather than what it meant to do.
    #[must_use]
    pub fn recorded(&self) -> &[Change] {
        &self.recorded
    }
}

#[cfg(test)]
#[path = "context_test.rs"]
mod context_test;
